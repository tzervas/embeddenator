"""
Runner Automation Package

Complete lifecycle automation for GitHub Actions self-hosted runners.
"""

__version__ = '1.1.0'

from .config import RunnerConfig
from .github_api import GitHubAPI
from .installer import RunnerInstaller
from .runner import Runner
from .manager import RunnerManager
from .emulation import EmulationManager

__all__ = [
    'RunnerConfig',
    'GitHubAPI',
    'RunnerInstaller',
    'Runner',
    'RunnerManager',
    'EmulationManager',
]
