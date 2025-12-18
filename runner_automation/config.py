"""
Runner Configuration Module

Handles configuration loading from environment variables and .env files.
"""

import os
import platform
from pathlib import Path
from typing import List


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
        self.fallback_version = os.getenv('RUNNER_FALLBACK_VERSION', '2.319.0')
        
        # Architecture configuration
        self.arch = os.getenv('RUNNER_ARCH', '') or self.detect_architecture()
        self.target_archs = self._parse_target_architectures()
        self.enable_emulation = os.getenv('RUNNER_ENABLE_EMULATION', 'true').lower() == 'true'
        self.emulation_auto_install = os.getenv('RUNNER_EMULATION_AUTO_INSTALL', 'false').lower() == 'true'
        
        # Logging
        self.log_level = os.getenv('LOG_LEVEL', 'INFO')
        self.log_file = Path(os.getenv('LOG_FILE', './runner_manager.log'))
        self.enable_metrics = os.getenv('ENABLE_METRICS', 'false').lower() == 'true'
        self.metrics_file = Path(os.getenv('METRICS_FILE', './runner_metrics.json'))
        
        # Timeouts
        self.api_timeout = int(os.getenv('GITHUB_API_TIMEOUT', '30'))
        self.version_check_timeout = int(os.getenv('GITHUB_VERSION_CHECK_TIMEOUT', '10'))
        
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
        elif machine in ('riscv64',):
            return 'riscv64'
        else:
            return 'x64'  # Default
    
    def _parse_target_architectures(self) -> List[str]:
        """
        Parse target architectures from environment
        
        Returns:
            List of target architectures to deploy
        """
        target_archs_str = os.getenv('RUNNER_TARGET_ARCHITECTURES', '').strip()
        
        if target_archs_str:
            # Parse comma-separated list
            archs = [arch.strip() for arch in target_archs_str.split(',')]
            return archs
        else:
            # Default to host architecture only
            return [self.arch]
    
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
