"""
Dynamic Runner Manager Module

Provides auto-scaling runner management with load balancing.
Automatically starts/stops runners based on demand.
"""

import logging
import time
import threading
from datetime import datetime, timedelta
from typing import Dict, List, Optional
from collections import defaultdict

from .runner import Runner
from .git_platforms import GitPlatformAPI


class RunnerPool:
    """Manages a pool of runners with different capabilities"""
    
    def __init__(self):
        self.runners: List[Runner] = []
        self.idle_runners: List[Runner] = []
        self.busy_runners: List[Runner] = []
        self.capabilities: Dict[str, List[Runner]] = defaultdict(list)
    
    def add_runner(self, runner: Runner, capabilities: List[str]):
        """Add runner to pool with its capabilities"""
        self.runners.append(runner)
        self.idle_runners.append(runner)
        for cap in capabilities:
            self.capabilities[cap].append(runner)
    
    def get_idle_runner(self, required_capabilities: List[str] = None) -> Optional[Runner]:
        """Get an idle runner matching required capabilities"""
        if not required_capabilities:
            return self.idle_runners[0] if self.idle_runners else None
        
        # Find runner with all required capabilities
        for runner in self.idle_runners:
            runner_caps = set()
            for cap, runners in self.capabilities.items():
                if runner in runners:
                    runner_caps.add(cap)
            
            if all(cap in runner_caps for cap in required_capabilities):
                return runner
        
        return None
    
    def mark_busy(self, runner: Runner):
        """Mark runner as busy"""
        if runner in self.idle_runners:
            self.idle_runners.remove(runner)
            self.busy_runners.append(runner)
    
    def mark_idle(self, runner: Runner):
        """Mark runner as idle"""
        if runner in self.busy_runners:
            self.busy_runners.remove(runner)
            self.idle_runners.append(runner)
    
    def remove_runner(self, runner: Runner):
        """Remove runner from pool"""
        if runner in self.runners:
            self.runners.remove(runner)
        if runner in self.idle_runners:
            self.idle_runners.remove(runner)
        if runner in self.busy_runners:
            self.busy_runners.remove(runner)
        
        # Remove from capabilities
        for cap_runners in self.capabilities.values():
            if runner in cap_runners:
                cap_runners.remove(runner)


class DynamicRunnerManager:
    """Manages runners dynamically with auto-scaling"""
    
    def __init__(self, config, git_platform: GitPlatformAPI, 
                 gpu_detector=None, resource_optimizer=None, emulation_mgr=None, logger=None):
        """
        Initialize dynamic runner manager
        
        Args:
            config: RunnerConfig instance
            git_platform: GitPlatformAPI instance
            gpu_detector: GPUDetector instance (optional)
            resource_optimizer: ResourceOptimizer instance (optional)
            emulation_mgr: EmulationManager instance (optional)
            logger: Logger instance
        """
        self.config = config
        self.git_platform = git_platform
        self.gpu_detector = gpu_detector
        self.resource_optimizer = resource_optimizer
        self.emulation_mgr = emulation_mgr
        self.logger = logger or logging.getLogger(__name__)
        
        self.pool = RunnerPool()
        self.next_runner_id = 1
        self.lock = threading.Lock()
        self.running = False
        self.monitor_thread = None
        
        # Auto-scaling configuration
        self.min_runners = config.min_runners if hasattr(config, 'min_runners') else 1
        self.max_runners = config.max_runners if hasattr(config, 'max_runners') else 10
        self.scale_up_threshold = config.scale_up_threshold if hasattr(config, 'scale_up_threshold') else 2
        self.scale_down_threshold = config.scale_down_threshold if hasattr(config, 'scale_down_threshold') else 0
        self.scale_cooldown = config.scale_cooldown if hasattr(config, 'scale_cooldown') else 60
        self.last_scale_time = datetime.now()
        
        # Capability tracking
        self.available_capabilities = self._detect_capabilities()
        
        self.logger.info("Dynamic Runner Manager initialized")
        self.logger.info(f"Auto-scaling: min={self.min_runners}, max={self.max_runners}")
        self.logger.info(f"Available capabilities: {', '.join(self.available_capabilities)}")
    
    def _detect_capabilities(self) -> List[str]:
        """Detect available capabilities based on system"""
        capabilities = ['amd64']  # Always have at least host architecture
        
        # Add architecture capabilities
        if self.config.target_archs:
            capabilities.extend(self.config.target_archs)
        
        # Add GPU capabilities
        if self.gpu_detector:
            gpus = self.gpu_detector.detect_all_gpus()
            if gpus:
                capabilities.append('gpu')
                for gpu in gpus:
                    capabilities.append(gpu.vendor)
                    if gpu.is_inference_capable:
                        capabilities.append('inference')
                    if gpu.is_training_capable:
                        capabilities.append('training')
        
        return list(set(capabilities))
    
    def start(self):
        """Start the dynamic runner manager"""
        self.logger.info("Starting dynamic runner manager...")
        
        # Start minimum runners
        self._scale_to(self.min_runners)
        
        # Start monitoring thread
        self.running = True
        self.monitor_thread = threading.Thread(target=self._monitor_loop, daemon=True)
        self.monitor_thread.start()
        
        self.logger.info("Dynamic runner manager started")
    
    def stop(self):
        """Stop the dynamic runner manager"""
        self.logger.info("Stopping dynamic runner manager...")
        self.running = False
        
        if self.monitor_thread:
            self.monitor_thread.join(timeout=10)
        
        # Stop all runners
        with self.lock:
            for runner in list(self.pool.runners):
                self._stop_runner(runner)
        
        self.logger.info("Dynamic runner manager stopped")
    
    def _monitor_loop(self):
        """Main monitoring loop for auto-scaling"""
        while self.running:
            try:
                self._check_and_scale()
                time.sleep(self.config.check_interval)
            except Exception as e:
                self.logger.error(f"Error in monitor loop: {e}")
                time.sleep(10)
    
    def _check_and_scale(self):
        """Check queue and scale runners as needed"""
        try:
            # Get pending jobs from platform
            pending_jobs = self.git_platform.get_pending_jobs()
            queue_length = len(pending_jobs)
            
            with self.lock:
                current_count = len(self.pool.runners)
                idle_count = len(self.pool.idle_runners)
                busy_count = len(self.pool.busy_runners)
            
            self.logger.debug(f"Queue: {queue_length}, Runners: {current_count} "
                            f"(idle: {idle_count}, busy: {busy_count})")
            
            # Check cooldown period
            if (datetime.now() - self.last_scale_time).total_seconds() < self.scale_cooldown:
                return
            
            # Scale up if queue is building
            if queue_length > self.scale_up_threshold and current_count < self.max_runners:
                # Calculate how many runners to add
                needed = min(
                    queue_length - idle_count,
                    self.max_runners - current_count
                )
                if needed > 0:
                    self.logger.info(f"Scaling up: adding {needed} runner(s) for {queue_length} pending jobs")
                    self._scale_up(needed)
                    self.last_scale_time = datetime.now()
            
            # Scale down if idle runners and no queue
            elif queue_length <= self.scale_down_threshold and idle_count > 1 and current_count > self.min_runners:
                # Keep minimum runners + 1 for quick response
                target_idle = 1
                to_remove = max(0, min(idle_count - target_idle, current_count - self.min_runners))
                if to_remove > 0:
                    self.logger.info(f"Scaling down: removing {to_remove} idle runner(s)")
                    self._scale_down(to_remove)
                    self.last_scale_time = datetime.now()
        
        except Exception as e:
            self.logger.error(f"Error checking scale: {e}")
    
    def _scale_up(self, count: int):
        """Scale up by adding runners"""
        for i in range(count):
            self._add_runner()
    
    def _scale_down(self, count: int):
        """Scale down by removing idle runners"""
        with self.lock:
            for i in range(count):
                if self.pool.idle_runners:
                    runner = self.pool.idle_runners[0]
                    self._stop_runner(runner)
    
    def _add_runner(self) -> Optional[Runner]:
        """Add a new runner to the pool"""
        with self.lock:
            runner_id = self.next_runner_id
            self.next_runner_id += 1
        
        # Determine architecture for this runner
        arch = self._select_architecture()
        
        # Get GPU assignment if available
        gpu = None
        if self.gpu_detector:
            gpus = self.gpu_detector.detect_all_gpus()
            # Assign GPU round-robin style
            if gpus:
                gpu_index = (runner_id - 1) % len(gpus)
                gpu = gpus[gpu_index]
        
        # Get resource limits
        resource_limits = None
        if self.resource_optimizer:
            allocation = self.resource_optimizer.calculate_optimal_resources(
                self.max_runners,
                enable_optimization=True,
                gpu_count=1 if gpu else 0
            )
            resource_limits = {
                'cpu_cores': allocation['cpu_cores_per_runner'],
                'memory_gb': allocation['memory_gb_per_runner'],
                'cpu_affinity': self.resource_optimizer.get_cpu_affinity(
                    runner_id,
                    allocation['cpu_cores_per_runner']
                ) if self.config.use_cpu_affinity else None
            }
        
        # Build labels with capabilities
        labels = self._build_labels(arch, gpu)
        
        # Create runner
        runner = Runner(
            self.config,
            self.git_platform,  # Use git platform instead of github_api
            self.logger,
            runner_id=runner_id,
            target_arch=arch,
            gpu_info=gpu,
            resource_limits=resource_limits,
            custom_labels=labels
        )
        
        # Register and start
        if runner.register():
            if runner.start():
                capabilities = self._extract_capabilities(labels)
                with self.lock:
                    self.pool.add_runner(runner, capabilities)
                
                self.logger.info(f"Added runner: {runner.name} with capabilities: {capabilities}")
                return runner
            else:
                runner.deregister()
        
        return None
    
    def _stop_runner(self, runner: Runner):
        """Stop and remove a runner"""
        self.logger.info(f"Stopping runner: {runner.name}")
        runner.stop()
        runner.deregister()
        runner.cleanup()
        
        with self.lock:
            self.pool.remove_runner(runner)
    
    def _select_architecture(self) -> str:
        """Select architecture for new runner"""
        # Simple round-robin for now
        if self.config.target_archs:
            with self.lock:
                index = len(self.pool.runners) % len(self.config.target_archs)
            return self.config.target_archs[index]
        return self.config.arch
    
    def _build_labels(self, arch: str, gpu=None) -> List[str]:
        """Build label list for runner"""
        labels = self.git_platform.get_runner_labels().copy()
        labels.append(arch)
        
        if gpu:
            gpu_labels = self.gpu_detector.get_gpu_labels(gpu)
            labels.extend(gpu_labels)
        
        return labels
    
    def _extract_capabilities(self, labels: List[str]) -> List[str]:
        """Extract capabilities from labels"""
        capabilities = []
        for label in labels:
            if label in ['amd64', 'arm64', 'x64', 'riscv64']:
                capabilities.append(label)
            elif label in ['gpu', 'inference', 'training']:
                capabilities.append(label)
            elif label in ['nvidia', 'amd', 'intel', 'apple']:
                capabilities.append(label)
        return capabilities
    
    def _scale_to(self, target: int):
        """Scale to specific number of runners"""
        with self.lock:
            current = len(self.pool.runners)
        
        if target > current:
            self._scale_up(target - current)
        elif target < current:
            self._scale_down(current - target)
    
    def get_status(self) -> Dict:
        """Get manager status"""
        with self.lock:
            return {
                'platform': self.git_platform.platform_name,
                'running': self.running,
                'total_runners': len(self.pool.runners),
                'idle_runners': len(self.pool.idle_runners),
                'busy_runners': len(self.pool.busy_runners),
                'min_runners': self.min_runners,
                'max_runners': self.max_runners,
                'capabilities': self.available_capabilities,
                'last_scale_time': self.last_scale_time.isoformat()
            }
