"""
Runner Manager Module

Manages multiple runners and handles lifecycle orchestration.
"""

import logging
import shutil
import signal
import subprocess
import time
from datetime import datetime
from typing import Dict, List

from .runner import Runner
from .emulation import EmulationManager
from .gpu_detection import GPUDetector
from .resource_optimizer import ResourceOptimizer


class RunnerManager:
    """Manage multiple runners and lifecycle"""
    
    def __init__(self, config, github_api):
        """
        Initialize runner manager
        
        Args:
            config: RunnerConfig instance
            github_api: GitHubAPI instance
        """
        self.config = config
        self.logger = self._setup_logger()
        self.github = github_api
        self.github.logger = self.logger  # Update GitHub API logger
        self.runners: List[Runner] = []
        self.shutdown_requested = False
        self.emulation_mgr = EmulationManager(self.logger, config.emulation_method)
        
        # Initialize GPU detection if enabled
        self.gpu_detector = None
        self.available_gpus = []
        if config.enable_gpu:
            self.gpu_detector = GPUDetector(self.logger)
            self.available_gpus = self.gpu_detector.detect_all_gpus()
            if self.available_gpus:
                self.logger.info(f"Detected {len(self.available_gpus)} GPU(s)")
                for gpu in self.available_gpus:
                    self.logger.info(f"  - {gpu}")
        
        # Initialize resource optimizer if enabled
        self.resource_optimizer = None
        self.resource_allocation = None
        if config.enable_resource_optimization:
            self.resource_optimizer = ResourceOptimizer(self.logger)
            gpu_count = len(self.available_gpus) if self.available_gpus else 0
            self.resource_allocation = self.resource_optimizer.calculate_optimal_resources(
                config.runner_count,
                enable_optimization=True,
                gpu_count=gpu_count
            )
            
            # Validate allocation
            is_valid, warnings = self.resource_optimizer.validate_allocation(
                self.resource_allocation,
                config.runner_count
            )
            if warnings:
                for warning in warnings:
                    self.logger.warning(f"Resource allocation: {warning}")
        
        # Setup signal handlers
        signal.signal(signal.SIGINT, self._handle_shutdown)
        signal.signal(signal.SIGTERM, self._handle_shutdown)
    
    def _setup_logger(self) -> logging.Logger:
        """
        Setup logging configuration
        
        Returns:
            Configured logger instance
        """
        logger = logging.getLogger('runner_manager')
        logger.setLevel(getattr(logging, self.config.log_level))
        
        # Console handler
        console_handler = logging.StreamHandler()
        console_handler.setLevel(logging.INFO)
        console_formatter = logging.Formatter('%(asctime)s [%(levelname)s] %(message)s')
        console_handler.setFormatter(console_formatter)
        logger.addHandler(console_handler)
        
        # File handler
        if self.config.log_file:
            file_handler = logging.FileHandler(self.config.log_file)
            file_handler.setLevel(getattr(logging, self.config.log_level))
            file_formatter = logging.Formatter('%(asctime)s [%(levelname)s] %(name)s - %(message)s')
            file_handler.setFormatter(file_formatter)
            logger.addHandler(file_handler)
        
        return logger
    
    def _handle_shutdown(self, signum, frame):
        """Handle shutdown signals"""
        self.logger.info(f"Received signal {signum}, initiating shutdown...")
        self.shutdown_requested = True
    
    def register_runners(self) -> bool:
        """
        Register all configured runners
        
        Returns:
            True if all successful, False otherwise
        """
        self.logger.info(f"Registering {self.config.runner_count} runner(s)...")
        
        # Setup emulation if needed
        if self.config.enable_emulation:
            self.logger.info("Checking emulation requirements...")
            for target_arch in self.config.target_archs:
                if self.emulation_mgr.is_emulation_needed(target_arch, self.config.arch):
                    self.logger.info(f"Emulation needed for {target_arch} on {self.config.arch}")
                    if not self.emulation_mgr.setup_emulation(target_arch, self.config.arch):
                        if not self.config.emulation_auto_install:
                            self.logger.error(f"Emulation setup failed for {target_arch}")
                            self.logger.error("Set RUNNER_EMULATION_AUTO_INSTALL=true to auto-install QEMU")
                            return False
                        else:
                            self.logger.error(f"Emulation setup failed for {target_arch} even with auto-install")
                            return False
                else:
                    self.logger.info(f"No emulation needed for {target_arch} on {self.config.arch}")
        
        # Determine which architectures to deploy
        archs_to_deploy = self.config.target_archs if self.config.target_archs else [self.config.arch]
        runners_per_arch = self.config.runner_count // len(archs_to_deploy)
        remainder = self.config.runner_count % len(archs_to_deploy)
        
        # Prepare GPU assignments if GPUs are available
        gpu_assignments = []
        if self.available_gpus:
            # Filter GPUs based on inference-only setting
            usable_gpus = self.available_gpus
            if self.config.inference_only:
                usable_gpus = self.gpu_detector.get_inference_gpus()
                self.logger.info(f"Inference-only mode: Using {len(usable_gpus)} inference-capable GPUs")
            
            # Distribute GPUs across runners
            for i in range(self.config.runner_count):
                if i < len(usable_gpus):
                    gpu_assignments.append(usable_gpus[i])
                else:
                    # Round-robin if more runners than GPUs
                    gpu_assignments.append(usable_gpus[i % len(usable_gpus)] if usable_gpus else None)
        else:
            gpu_assignments = [None] * self.config.runner_count
        
        runner_id = 1
        for arch_idx, arch in enumerate(archs_to_deploy):
            # Distribute remainder across first architectures
            count_for_arch = runners_per_arch + (1 if arch_idx < remainder else 0)
            
            self.logger.info(f"Deploying {count_for_arch} runner(s) for {arch}")
            
            for i in range(count_for_arch):
                # Get GPU assignment for this runner
                gpu = gpu_assignments[runner_id - 1] if runner_id <= len(gpu_assignments) else None
                
                # Get resource allocation if optimization is enabled
                resource_limits = None
                if self.resource_allocation:
                    resource_limits = {
                        'cpu_cores': self.resource_allocation['cpu_cores_per_runner'],
                        'memory_gb': self.resource_allocation['memory_gb_per_runner'],
                        'cpu_affinity': self.resource_optimizer.get_cpu_affinity(
                            runner_id,
                            self.resource_allocation['cpu_cores_per_runner']
                        ) if self.config.use_cpu_affinity else None
                    }
                
                # Add GPU-specific labels if GPU is assigned
                custom_labels = self.config.labels.copy()
                if gpu:
                    gpu_labels = self.gpu_detector.get_gpu_labels(gpu)
                    custom_labels.extend(gpu_labels)
                    self.logger.info(f"Runner {runner_id} assigned to {gpu} with labels: {gpu_labels}")
                
                # Create runner with specific architecture and GPU
                runner = Runner(
                    self.config, 
                    self.github, 
                    self.logger, 
                    runner_id=runner_id, 
                    target_arch=arch,
                    gpu_info=gpu,
                    resource_limits=resource_limits,
                    custom_labels=custom_labels
                )
                
                if not runner.register():
                    self.logger.error(f"Failed to register runner {runner_id} ({arch})")
                    return False
                
                self.runners.append(runner)
                runner_id += 1
                
                # Stagger deployments if sequential
                if self.config.deployment_strategy == 'sequential' and runner_id <= self.config.runner_count:
                    self.logger.info(f"Waiting {self.config.deployment_stagger}s before next runner...")
                    time.sleep(self.config.deployment_stagger)
        
        self.logger.info("All runners registered successfully")
        return True
    
    def start_runners(self) -> bool:
        """
        Start all runner processes
        
        Returns:
            True if all successful, False otherwise
        """
        self.logger.info("Starting runner processes...")
        
        if not self.runners:
            self.logger.error("No runners to start. Run 'register' first.")
            return False
        
        for runner in self.runners:
            if not runner.start():
                self.logger.error(f"Failed to start runner {runner.name}")
                return False
        
        self.logger.info("All runners started successfully")
        return True
    
    def stop_runners(self) -> bool:
        """
        Stop all runner processes
        
        Returns:
            True if all successful, False otherwise
        """
        self.logger.info("Stopping runner processes...")
        
        success = True
        for runner in self.runners:
            if not runner.stop():
                success = False
        
        return success
    
    def deregister_runners(self) -> bool:
        """
        Deregister all runners
        
        Returns:
            True if all successful, False otherwise
        """
        self.logger.info("Deregistering runners...")
        
        success = True
        for runner in self.runners:
            if not runner.deregister():
                success = False
            runner.cleanup()
        
        if self.config.clean_docker:
            self._cleanup_docker()
        
        return success
    
    def _cleanup_docker(self):
        """Clean up Docker resources"""
        self.logger.info("Cleaning up Docker resources...")
        
        try:
            # Check disk space using shutil for cross-platform compatibility
            disk_usage = shutil.disk_usage('/')
            available_gb = disk_usage.free / (1024**3)
            
            if available_gb < self.config.docker_cleanup_threshold_gb:
                self.logger.warning(f"Low disk space: {available_gb:.1f}GB, running Docker cleanup")
                subprocess.run(['docker', 'system', 'prune', '-a', '-f'], check=False)
            else:
                self.logger.debug(f"Disk space OK: {available_gb:.1f}GB available")
        except Exception as e:
            self.logger.warning(f"Docker cleanup failed: {e}")
    
    def monitor_lifecycle(self):
        """Monitor runner lifecycle and handle auto-deregistration"""
        self.logger.info(f"Starting lifecycle monitoring (mode: {self.config.mode})")
        
        if self.config.mode == 'manual':
            self.logger.info("Manual mode: runners will run until manually stopped")
            self._monitor_manual_mode()
        else:
            self.logger.info(f"Auto mode: idle timeout = {self.config.idle_timeout}s")
            self._monitor_auto_mode()
    
    def _monitor_manual_mode(self):
        """Monitor in manual mode - run until stopped"""
        while not self.shutdown_requested:
            # Check runner health
            for runner in self.runners:
                if not runner.is_running():
                    self.logger.warning(f"Runner {runner.name} stopped unexpectedly")
            
            time.sleep(self.config.check_interval)
        
        self.logger.info("Shutdown requested in manual mode")
    
    def _monitor_auto_mode(self):
        """Monitor in auto mode - deregister after idle timeout"""
        idle_start = None
        
        while not self.shutdown_requested:
            # Check for queued jobs
            try:
                queued_jobs = self.github.count_queued_jobs()
                self.logger.debug(f"Queued jobs: {queued_jobs}")
                
                if queued_jobs > 0:
                    idle_start = None
                    self.logger.debug("Jobs in queue, resetting idle timer")
                else:
                    if idle_start is None:
                        idle_start = datetime.now()
                        self.logger.info("No jobs in queue, starting idle timer")
                    else:
                        idle_duration = (datetime.now() - idle_start).total_seconds()
                        self.logger.debug(f"Idle for {idle_duration:.0f}s / {self.config.idle_timeout}s")
                        
                        if idle_duration >= self.config.idle_timeout:
                            self.logger.info(f"Idle timeout reached ({self.config.idle_timeout}s), initiating shutdown")
                            break
            except Exception as e:
                self.logger.error(f"Error checking queue: {e}")
            
            # Check max lifetime
            if self.config.max_lifetime > 0:
                for runner in self.runners:
                    if runner.start_time:
                        lifetime = (datetime.now() - runner.start_time).total_seconds()
                        if lifetime >= self.config.max_lifetime:
                            self.logger.info(f"Max lifetime reached ({self.config.max_lifetime}s), initiating shutdown")
                            self.shutdown_requested = True
                            break
            
            # Check runner health
            for runner in self.runners:
                if not runner.is_running():
                    self.logger.warning(f"Runner {runner.name} stopped unexpectedly")
            
            time.sleep(self.config.check_interval)
    
    def get_status(self) -> Dict:
        """
        Get status of all runners
        
        Returns:
            Dictionary with manager and runner status
        """
        status = {
            'manager': {
                'mode': self.config.mode,
                'runner_count': len(self.runners),
                'shutdown_requested': self.shutdown_requested
            },
            'runners': [runner.get_status() for runner in self.runners]
        }
        
        # Add queue info
        try:
            status['queue'] = {
                'queued_jobs': self.github.count_queued_jobs()
            }
        except Exception as e:
            self.logger.error(f"Failed to get queue info: {e}")
        
        return status
    
    def print_status(self):
        """Print formatted status information"""
        status = self.get_status()
        
        print("\n" + "=" * 70)
        print("GitHub Actions Runner Manager - Status")
        print("=" * 70)
        print(f"\nMode: {status['manager']['mode']}")
        print(f"Runner Count: {status['manager']['runner_count']}")
        print(f"Repository: {self.config.repository}")
        
        if 'queue' in status:
            print(f"\nQueued Jobs: {status['queue'].get('queued_jobs', 'unknown')}")
        
        print("\nRunners:")
        print("-" * 70)
        for runner_status in status['runners']:
            print(f"\n  Name: {runner_status['name']}")
            print(f"  Running: {runner_status['running']}")
            print(f"  PID: {runner_status.get('pid', 'N/A')}")
            print(f"  Uptime: {runner_status.get('uptime', 'N/A')}")
            if 'github_status' in runner_status:
                print(f"  GitHub Status: {runner_status['github_status']}")
                print(f"  Busy: {runner_status.get('github_busy', 'N/A')}")
        
        print("\n" + "=" * 70 + "\n")
