"""
Runner Installer Module

Handles downloading and installing GitHub Actions runner software.
"""

import json
import logging
import shutil
import subprocess
import urllib.request
from pathlib import Path


class RunnerInstaller:
    """Handle runner installation and setup"""
    
    RUNNER_DOWNLOAD_URLS = {
        'x64': 'https://github.com/actions/runner/releases/download/v{version}/actions-runner-linux-x64-{version}.tar.gz',
        'arm64': 'https://github.com/actions/runner/releases/download/v{version}/actions-runner-linux-arm64-{version}.tar.gz',
        # Note: RISC-V runners may not be officially available yet, fallback to x64 with emulation
        'riscv64': 'https://github.com/actions/runner/releases/download/v{version}/actions-runner-linux-x64-{version}.tar.gz'
    }
    
    def __init__(self, config, logger: logging.Logger):
        """
        Initialize runner installer
        
        Args:
            config: RunnerConfig instance
            logger: Logger instance
        """
        self.config = config
        self.logger = logger
    
    def get_latest_version(self) -> str:
        """
        Get the latest runner version from GitHub
        
        Returns:
            Version string (e.g., '2.319.0')
        """
        try:
            url = 'https://api.github.com/repos/actions/runner/releases/latest'
            req = urllib.request.Request(url, headers={'User-Agent': 'embeddenator-runner-manager'})
            with urllib.request.urlopen(req, timeout=self.config.version_check_timeout) as response:
                data = json.loads(response.read().decode('utf-8'))
                return data['tag_name'].lstrip('v')
        except Exception as e:
            self.logger.warning(f"Failed to get latest version: {e}, using {self.config.fallback_version}")
            return self.config.fallback_version
    
    def install(self, install_dir: Path) -> bool:
        """
        Install GitHub Actions runner
        
        Args:
            install_dir: Directory to install runner into
            
        Returns:
            True if successful, False otherwise
        """
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
