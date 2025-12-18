#!/usr/bin/env python3
"""
GitHub Actions Self-Hosted Runner Manager

Automates the complete lifecycle of GitHub Actions self-hosted runners:
- Registration with short-lived tokens
- Job monitoring and queue tracking
- Automatic deregistration after idle timeout
- Manual mode support (keep-alive until stopped)
- Multi-runner deployment support
- Health monitoring and status reporting

Usage:
    python3 runner_manager.py register    # Register runner(s)
    python3 runner_manager.py start       # Start runner service
    python3 runner_manager.py stop        # Stop and deregister
    python3 runner_manager.py status      # Show status
    python3 runner_manager.py monitor     # Continuous monitoring
"""

import argparse
import json
import logging
import os
import platform
import shutil
import signal
import subprocess
import sys
import time
from datetime import datetime, timedelta
from pathlib import Path
from typing import Dict, List, Optional, Tuple
import urllib.request
import urllib.error


class RunnerConfig:
    """Configuration manager for runner settings"""
    
    def __init__(self):
        """Initialize configuration from environment and .env file"""
        self.load_env_file()
        
        # GitHub configuration
        self.repository = os.getenv('GITHUB_REPOSITORY', '')
        self.api_url = os.getenv('GITHUB_API_URL', 'https://api.github.com')
        self.token = os.getenv('GITHUB_TOKEN', '')
        
        # Runner configuration
        self.name_prefix = os.getenv('RUNNER_NAME_PREFIX', 'embeddenator-runner')
        self.labels = os.getenv('RUNNER_LABELS', 'self-hosted,linux,ARM64').split(',')
        self.group = os.getenv('RUNNER_GROUP', 'Default')
        self.work_dir = os.getenv('RUNNER_WORK_DIR', '_work')
        
        # Lifecycle management
        self.mode = os.getenv('RUNNER_MODE', 'auto')
        self.idle_timeout = int(os.getenv('RUNNER_IDLE_TIMEOUT', '300'))
        self.check_interval = int(os.getenv('RUNNER_CHECK_INTERVAL', '30'))
        self.max_lifetime = int(os.getenv('RUNNER_MAX_LIFETIME', '0'))
        
        # Multi-runner deployment
        self.runner_count = int(os.getenv('RUNNER_COUNT', '1'))
        self.deployment_strategy = os.getenv('RUNNER_DEPLOYMENT_STRATEGY', 'sequential')
        self.deployment_stagger = int(os.getenv('RUNNER_DEPLOYMENT_STAGGER', '5'))
        
        # Resources
        self.cpu_cores = os.getenv('RUNNER_CPU_CORES', '')
        self.memory_gb = os.getenv('RUNNER_MEMORY_GB', '')
        self.disk_threshold_gb = int(os.getenv('RUNNER_DISK_THRESHOLD_GB', '20'))
        
        # Installation
        self.install_dir = Path(os.getenv('RUNNER_INSTALL_DIR', './actions-runner'))
        self.version = os.getenv('RUNNER_VERSION', 'latest')
        self.arch = os.getenv('RUNNER_ARCH', '') or self.detect_architecture()
        
        # Logging
        self.log_level = os.getenv('LOG_LEVEL', 'INFO')
        self.log_file = Path(os.getenv('LOG_FILE', './runner_manager.log'))
        self.enable_metrics = os.getenv('ENABLE_METRICS', 'false').lower() == 'true'
        self.metrics_file = Path(os.getenv('METRICS_FILE', './runner_metrics.json'))
        
        # Advanced
        self.ephemeral = os.getenv('RUNNER_EPHEMERAL', 'false').lower() == 'true'
        self.replace_existing = os.getenv('RUNNER_REPLACE_EXISTING', 'false').lower() == 'true'
        self.disable_auto_update = os.getenv('RUNNER_DISABLE_AUTO_UPDATE', 'false').lower() == 'true'
        additional_flags_str = os.getenv('RUNNER_ADDITIONAL_FLAGS', '').strip()
        self.additional_flags = additional_flags_str.split() if additional_flags_str else []
        
        # Cleanup
        self.clean_on_deregister = os.getenv('RUNNER_CLEAN_ON_DEREGISTER', 'true').lower() == 'true'
        self.clean_docker = os.getenv('RUNNER_CLEAN_DOCKER', 'true').lower() == 'true'
        self.docker_cleanup_threshold_gb = int(os.getenv('DOCKER_CLEANUP_THRESHOLD_GB', '10'))
        
    def load_env_file(self):
        """Load environment variables from .env file if it exists"""
        env_file = Path('.env')
        if env_file.exists():
            with open(env_file) as f:
                for line in f:
                    line = line.strip()
                    if line and not line.startswith('#') and '=' in line:
                        key, value = line.split('=', 1)
                        key = key.strip()
                        value = value.strip()
                        if key and value:
                            # Always load from .env, but environment variables take precedence
                            # Only set if not already in environment
                            if key not in os.environ:
                                os.environ[key] = value
    
    def detect_architecture(self) -> str:
        """Detect system architecture"""
        machine = platform.machine().lower()
        if machine in ('x86_64', 'amd64'):
            return 'x64'
        elif machine in ('arm64', 'aarch64'):
            return 'arm64'
        else:
            return 'x64'  # Default
    
    def validate(self) -> List[str]:
        """Validate configuration and return list of errors"""
        errors = []
        
        if not self.repository:
            errors.append("GITHUB_REPOSITORY is required")
        
        if not self.token:
            errors.append("GITHUB_TOKEN is required")
        
        if self.mode not in ('auto', 'manual'):
            errors.append(f"Invalid RUNNER_MODE: {self.mode} (must be 'auto' or 'manual')")
        
        if self.deployment_strategy not in ('sequential', 'parallel'):
            errors.append(f"Invalid RUNNER_DEPLOYMENT_STRATEGY: {self.deployment_strategy}")
        
        if self.runner_count < 1:
            errors.append(f"Invalid RUNNER_COUNT: {self.runner_count} (must be >= 1)")
        
        return errors


class GitHubAPI:
    """GitHub API client for runner management"""
    
    def __init__(self, config: RunnerConfig, logger: logging.Logger):
        self.config = config
        self.logger = logger
    
    def _make_request(self, endpoint: str, method: str = 'GET', data: Optional[Dict] = None) -> Dict:
        """Make an authenticated request to GitHub API"""
        url = f"{self.config.api_url}/repos/{self.config.repository}/{endpoint}"
        
        headers = {
            'Authorization': f'token {self.config.token}',
            'Accept': 'application/vnd.github.v3+json',
            'User-Agent': 'embeddenator-runner-manager'
        }
        
        if data:
            data = json.dumps(data).encode('utf-8')
            headers['Content-Type'] = 'application/json'
        
        req = urllib.request.Request(url, data=data, headers=headers, method=method)
        
        try:
            with urllib.request.urlopen(req, timeout=30) as response:
                return json.loads(response.read().decode('utf-8'))
        except urllib.error.HTTPError as e:
            error_msg = e.read().decode('utf-8') if e.fp else str(e)
            self.logger.error(f"GitHub API error: {e.code} - {error_msg}")
            raise
        except Exception as e:
            self.logger.error(f"Request failed: {e}")
            raise
    
    def get_registration_token(self) -> str:
        """Get a short-lived registration token from GitHub"""
        self.logger.info("Obtaining registration token from GitHub...")
        response = self._make_request('actions/runners/registration-token', method='POST')
        token = response.get('token')
        expires_at = response.get('expires_at')
        
        self.logger.info(f"Registration token obtained (expires: {expires_at})")
        return token
    
    def get_removal_token(self) -> str:
        """Get a removal token from GitHub"""
        self.logger.info("Obtaining removal token from GitHub...")
        response = self._make_request('actions/runners/remove-token', method='POST')
        return response.get('token')
    
    def list_runners(self) -> List[Dict]:
        """List all runners for the repository"""
        response = self._make_request('actions/runners')
        return response.get('runners', [])
    
    def get_runner_by_name(self, name: str) -> Optional[Dict]:
        """Find a runner by name"""
        runners = self.list_runners()
        for runner in runners:
            if runner.get('name') == name:
                return runner
        return None
    
    def get_workflow_runs(self, status: str = 'queued') -> List[Dict]:
        """Get workflow runs with specified status"""
        try:
            response = self._make_request(f'actions/runs?status={status}')
            return response.get('workflow_runs', [])
        except Exception as e:
            self.logger.warning(f"Failed to get workflow runs: {e}")
            return []
    
    def count_queued_jobs(self) -> int:
        """Count jobs currently in queue"""
        queued_runs = self.get_workflow_runs('queued')
        in_progress_runs = self.get_workflow_runs('in_progress')
        return len(queued_runs) + len(in_progress_runs)


class RunnerInstaller:
    """Handle runner installation and setup"""
    
    RUNNER_DOWNLOAD_URLS = {
        'x64': 'https://github.com/actions/runner/releases/download/v{version}/actions-runner-linux-x64-{version}.tar.gz',
        'arm64': 'https://github.com/actions/runner/releases/download/v{version}/actions-runner-linux-arm64-{version}.tar.gz'
    }
    
    def __init__(self, config: RunnerConfig, logger: logging.Logger):
        self.config = config
        self.logger = logger
    
    def get_latest_version(self) -> str:
        """Get the latest runner version from GitHub"""
        try:
            url = 'https://api.github.com/repos/actions/runner/releases/latest'
            req = urllib.request.Request(url, headers={'User-Agent': 'embeddenator-runner-manager'})
            with urllib.request.urlopen(req, timeout=10) as response:
                data = json.loads(response.read().decode('utf-8'))
                return data['tag_name'].lstrip('v')
        except Exception as e:
            self.logger.warning(f"Failed to get latest version: {e}, using 2.311.0")
            return '2.311.0'
    
    def install(self, install_dir: Path) -> bool:
        """Install GitHub Actions runner"""
        if install_dir.exists():
            self.logger.info(f"Runner already installed at {install_dir}")
            return True
        
        install_dir.mkdir(parents=True, exist_ok=True)
        
        version = self.config.version
        if version == 'latest':
            version = self.get_latest_version()
        
        url_template = self.RUNNER_DOWNLOAD_URLS.get(self.config.arch)
        if not url_template:
            self.logger.error(f"Unsupported architecture: {self.config.arch}")
            return False
        
        url = url_template.format(version=version)
        tarball = install_dir / 'runner.tar.gz'
        
        self.logger.info(f"Downloading runner version {version} for {self.config.arch}...")
        self.logger.info(f"URL: {url}")
        
        try:
            urllib.request.urlretrieve(url, tarball)
            self.logger.info(f"Downloaded to {tarball}")
            
            self.logger.info("Extracting runner...")
            subprocess.run(['tar', 'xzf', str(tarball), '-C', str(install_dir)], check=True)
            tarball.unlink()
            
            self.logger.info("Runner installation complete")
            return True
        except Exception as e:
            self.logger.error(f"Installation failed: {e}")
            if install_dir.exists():
                shutil.rmtree(install_dir)
            return False


class Runner:
    """Manage individual runner lifecycle"""
    
    def __init__(self, config: RunnerConfig, github: GitHubAPI, logger: logging.Logger, 
                 runner_id: int = 1):
        self.config = config
        self.github = github
        self.logger = logger
        self.runner_id = runner_id
        self.name = f"{config.name_prefix}-{runner_id}"
        self.install_dir = config.install_dir.parent / f"{config.install_dir.name}-{runner_id}"
        self.process = None
        self.start_time = None
        self.last_activity = None
    
    def register(self) -> bool:
        """Register runner with GitHub"""
        self.logger.info(f"Registering runner: {self.name}")
        
        # Install runner if needed
        installer = RunnerInstaller(self.config, self.logger)
        if not installer.install(self.install_dir):
            return False
        
        # Get registration token
        try:
            token = self.github.get_registration_token()
        except Exception as e:
            self.logger.error(f"Failed to get registration token: {e}")
            return False
        
        # Build config command
        config_script = self.install_dir / 'config.sh'
        if not config_script.exists():
            self.logger.error(f"config.sh not found at {config_script}")
            return False
        
        cmd = [
            str(config_script),
            '--url', f'https://github.com/{self.config.repository}',
            '--token', token,
            '--name', self.name,
            '--labels', ','.join(self.config.labels),
            '--work', self.config.work_dir,
            '--unattended'
        ]
        
        if self.config.replace_existing:
            cmd.append('--replace')
        
        if self.config.ephemeral:
            cmd.append('--ephemeral')
        
        if self.config.disable_auto_update:
            cmd.append('--disableupdate')
        
        cmd.extend(self.config.additional_flags)
        
        self.logger.debug(f"Running: {' '.join(cmd[:6])}...")  # Don't log token
        
        try:
            result = subprocess.run(cmd, cwd=self.install_dir, capture_output=True, text=True)
            if result.returncode == 0:
                self.logger.info(f"Runner {self.name} registered successfully")
                return True
            else:
                self.logger.error(f"Registration failed: {result.stderr}")
                return False
        except Exception as e:
            self.logger.error(f"Registration failed: {e}")
            return False
    
    def start(self) -> bool:
        """Start the runner process"""
        run_script = self.install_dir / 'run.sh'
        if not run_script.exists():
            self.logger.error(f"run.sh not found at {run_script}")
            return False
        
        self.logger.info(f"Starting runner: {self.name}")
        
        try:
            self.process = subprocess.Popen(
                [str(run_script)],
                cwd=self.install_dir,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                text=True
            )
            self.start_time = datetime.now()
            self.last_activity = self.start_time
            self.logger.info(f"Runner {self.name} started (PID: {self.process.pid})")
            return True
        except Exception as e:
            self.logger.error(f"Failed to start runner: {e}")
            return False
    
    def stop(self) -> bool:
        """Stop the runner process"""
        if not self.process:
            return True
        
        self.logger.info(f"Stopping runner: {self.name}")
        
        try:
            self.process.terminate()
            self.process.wait(timeout=30)
            self.logger.info(f"Runner {self.name} stopped")
            return True
        except subprocess.TimeoutExpired:
            self.logger.warning(f"Runner {self.name} did not stop gracefully, killing...")
            self.process.kill()
            self.process.wait()
            return True
        except Exception as e:
            self.logger.error(f"Failed to stop runner: {e}")
            return False
    
    def deregister(self) -> bool:
        """Deregister runner from GitHub"""
        self.logger.info(f"Deregistering runner: {self.name}")
        
        config_script = self.install_dir / 'config.sh'
        if not config_script.exists():
            self.logger.warning(f"config.sh not found, skipping deregistration")
            return True
        
        try:
            token = self.github.get_removal_token()
        except Exception as e:
            self.logger.error(f"Failed to get removal token: {e}")
            return False
        
        cmd = [
            str(config_script),
            'remove',
            '--token', token
        ]
        
        try:
            result = subprocess.run(cmd, cwd=self.install_dir, capture_output=True, text=True)
            if result.returncode == 0:
                self.logger.info(f"Runner {self.name} deregistered successfully")
                return True
            else:
                self.logger.error(f"Deregistration failed: {result.stderr}")
                return False
        except Exception as e:
            self.logger.error(f"Deregistration failed: {e}")
            return False
    
    def cleanup(self):
        """Clean up runner installation"""
        if self.config.clean_on_deregister and self.install_dir.exists():
            self.logger.info(f"Cleaning up runner installation: {self.install_dir}")
            try:
                shutil.rmtree(self.install_dir)
            except Exception as e:
                self.logger.error(f"Failed to cleanup: {e}")
    
    def is_running(self) -> bool:
        """Check if runner process is running"""
        if not self.process:
            return False
        return self.process.poll() is None
    
    def get_status(self) -> Dict:
        """Get runner status information"""
        status = {
            'name': self.name,
            'runner_id': self.runner_id,
            'install_dir': str(self.install_dir),
            'running': self.is_running(),
            'pid': self.process.pid if self.process else None,
            'start_time': self.start_time.isoformat() if self.start_time else None,
            'uptime': str(datetime.now() - self.start_time) if self.start_time else None
        }
        
        # Check GitHub status
        try:
            runner_info = self.github.get_runner_by_name(self.name)
            if runner_info:
                status['github_status'] = runner_info.get('status')
                status['github_busy'] = runner_info.get('busy')
        except Exception:
            pass
        
        return status


class RunnerManager:
    """Manage multiple runners and lifecycle"""
    
    def __init__(self, config: RunnerConfig):
        self.config = config
        self.logger = self._setup_logger()
        self.github = GitHubAPI(config, self.logger)
        self.runners: List[Runner] = []
        self.shutdown_requested = False
        
        # Setup signal handlers
        signal.signal(signal.SIGINT, self._handle_shutdown)
        signal.signal(signal.SIGTERM, self._handle_shutdown)
    
    def _setup_logger(self) -> logging.Logger:
        """Setup logging configuration"""
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
        """Register all configured runners"""
        self.logger.info(f"Registering {self.config.runner_count} runner(s)...")
        
        for i in range(1, self.config.runner_count + 1):
            runner = Runner(self.config, self.github, self.logger, runner_id=i)
            
            if not runner.register():
                self.logger.error(f"Failed to register runner {i}")
                return False
            
            self.runners.append(runner)
            
            # Stagger deployments if sequential
            if self.config.deployment_strategy == 'sequential' and i < self.config.runner_count:
                self.logger.info(f"Waiting {self.config.deployment_stagger}s before next runner...")
                time.sleep(self.config.deployment_stagger)
        
        self.logger.info("All runners registered successfully")
        return True
    
    def start_runners(self) -> bool:
        """Start all runner processes"""
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
        """Stop all runner processes"""
        self.logger.info("Stopping runner processes...")
        
        success = True
        for runner in self.runners:
            if not runner.stop():
                success = False
        
        return success
    
    def deregister_runners(self) -> bool:
        """Deregister all runners"""
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
        """Get status of all runners"""
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


def main():
    """Main entry point"""
    parser = argparse.ArgumentParser(
        description='GitHub Actions Self-Hosted Runner Manager',
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
Examples:
  # Register and start runner in auto mode
  python3 runner_manager.py register
  python3 runner_manager.py start
  python3 runner_manager.py monitor
  
  # Quick start (register + start + monitor)
  python3 runner_manager.py run
  
  # Check status
  python3 runner_manager.py status
  
  # Stop and deregister
  python3 runner_manager.py stop
  
  # Override config with CLI arguments
  python3 runner_manager.py register --runner-count 4 --labels self-hosted,linux,ARM64,large
  
  # Manual mode (keep running until stopped)
  RUNNER_MODE=manual python3 runner_manager.py run
        """
    )
    
    subparsers = parser.add_subparsers(dest='command', help='Command to execute')
    
    # Register command
    register_parser = subparsers.add_parser('register', help='Register runner(s) with GitHub')
    register_parser.add_argument('--runner-count', type=int, help='Number of runners to deploy')
    register_parser.add_argument('--labels', help='Runner labels (comma-separated)')
    
    # Start command
    start_parser = subparsers.add_parser('start', help='Start runner process(es)')
    
    # Stop command
    stop_parser = subparsers.add_parser('stop', help='Stop and deregister runner(s)')
    
    # Status command
    status_parser = subparsers.add_parser('status', help='Show runner status')
    
    # Monitor command
    monitor_parser = subparsers.add_parser('monitor', help='Monitor runners and manage lifecycle')
    
    # Run command (register + start + monitor)
    run_parser = subparsers.add_parser('run', help='Register, start, and monitor runners (all-in-one)')
    run_parser.add_argument('--runner-count', type=int, help='Number of runners to deploy')
    run_parser.add_argument('--labels', help='Runner labels (comma-separated)')
    
    args = parser.parse_args()
    
    if not args.command:
        parser.print_help()
        return 1
    
    # Load configuration
    config = RunnerConfig()
    
    # Override with CLI arguments
    if hasattr(args, 'runner_count') and args.runner_count:
        config.runner_count = args.runner_count
    if hasattr(args, 'labels') and args.labels:
        config.labels = args.labels.split(',')
    
    # Validate configuration
    errors = config.validate()
    if errors:
        print("Configuration errors:", file=sys.stderr)
        for error in errors:
            print(f"  - {error}", file=sys.stderr)
        return 1
    
    # Create manager
    manager = RunnerManager(config)
    
    try:
        if args.command == 'register':
            success = manager.register_runners()
            return 0 if success else 1
        
        elif args.command == 'start':
            success = manager.start_runners()
            return 0 if success else 1
        
        elif args.command == 'stop':
            manager.stop_runners()
            success = manager.deregister_runners()
            return 0 if success else 1
        
        elif args.command == 'status':
            manager.print_status()
            return 0
        
        elif args.command == 'monitor':
            if not manager.runners:
                print("Error: No runners found. Run 'register' and 'start' first.", file=sys.stderr)
                return 1
            manager.monitor_lifecycle()
            # Cleanup on exit
            manager.stop_runners()
            manager.deregister_runners()
            return 0
        
        elif args.command == 'run':
            # All-in-one: register, start, monitor
            if not manager.register_runners():
                return 1
            if not manager.start_runners():
                return 1
            manager.monitor_lifecycle()
            # Cleanup on exit
            manager.stop_runners()
            manager.deregister_runners()
            return 0
        
        else:
            parser.print_help()
            return 1
    
    except KeyboardInterrupt:
        print("\nInterrupted by user")
        manager.stop_runners()
        manager.deregister_runners()
        return 130
    except Exception as e:
        manager.logger.error(f"Fatal error: {e}", exc_info=True)
        return 1


if __name__ == '__main__':
    sys.exit(main())
