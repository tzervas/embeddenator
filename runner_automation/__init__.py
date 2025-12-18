"""
Runner Automation Package

Complete lifecycle automation for GitHub Actions self-hosted runners.
"""

__version__ = '1.4.0'

from .config import RunnerConfig
from .github_api import GitHubAPI
from .installer import RunnerInstaller
from .runner import Runner
from .manager import RunnerManager
from .emulation import EmulationManager, ContainerRuntime
from .gpu_detection import GPUDetector, GPUInfo
from .resource_optimizer import ResourceOptimizer, CPUInfo
from .git_platforms import GitPlatformFactory, GitPlatformAPI, GitHubPlatform, GitLabPlatform, GiteaPlatform
from .dynamic_manager import DynamicRunnerManager, RunnerPool

__all__ = [
    'RunnerConfig',
    'GitHubAPI',
    'RunnerInstaller',
    'Runner',
    'RunnerManager',
    'EmulationManager',
    'ContainerRuntime',
    'GPUDetector',
    'GPUInfo',
    'ResourceOptimizer',
    'CPUInfo',
    'GitPlatformFactory',
    'GitPlatformAPI',
    'GitHubPlatform',
    'GitLabPlatform',
    'GiteaPlatform',
    'DynamicRunnerManager',
    'RunnerPool',
]
