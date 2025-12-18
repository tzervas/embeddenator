"""
Runner Module

Manages individual runner lifecycle (register, start, stop, deregister).
"""

import logging
import shutil
import subprocess
from datetime import datetime
from pathlib import Path
from typing import Dict, Optional

from .installer import RunnerInstaller


class Runner:
    """Manage individual runner lifecycle"""
    
    def __init__(self, config, github_api, logger: logging.Logger, runner_id: int = 1, target_arch: Optional[str] = None):
        """
        Initialize runner
        
        Args:
            config: RunnerConfig instance
            github_api: GitHubAPI instance
            logger: Logger instance
            runner_id: Unique ID for this runner
            target_arch: Target architecture (overrides config.arch if provided)
        """
        self.config = config
        self.github = github_api
        self.logger = logger
        self.runner_id = runner_id
        self.target_arch = target_arch or config.arch
        self.name = f"{config.name_prefix}-{self.target_arch}-{runner_id}"
        self.install_dir = config.install_dir.parent / f"{config.install_dir.name}-{self.target_arch}-{runner_id}"
        self.process = None
        self.start_time = None
        self.last_activity = None
    
    def register(self) -> bool:
        """
        Register runner with GitHub
        
        Returns:
            True if successful, False otherwise
        """
        self.logger.info(f"Registering runner: {self.name} ({self.target_arch})")
        
        # Install runner if needed - pass target_arch to installer
        installer = RunnerInstaller(self.config, self.logger)
        # Temporarily override config arch for this specific runner
        original_arch = self.config.arch
        self.config.arch = self.target_arch
        success = installer.install(self.install_dir)
        self.config.arch = original_arch
        
        if not success:
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
        """
        Start the runner process
        
        Returns:
            True if successful, False otherwise
        """
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
        """
        Stop the runner process
        
        Returns:
            True if successful, False otherwise
        """
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
        """
        Deregister runner from GitHub
        
        Returns:
            True if successful, False otherwise
        """
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
        """
        Check if runner process is running
        
        Returns:
            True if running, False otherwise
        """
        if not self.process:
            return False
        return self.process.poll() is None
    
    def get_status(self) -> Dict:
        """
        Get runner status information
        
        Returns:
            Dictionary with status information
        """
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
