"""
Git Platform Abstraction Module

Provides unified interface for multiple git platforms (GitHub, GitLab, Gitea).
"""

import json
import logging
import urllib.request
import urllib.error
from abc import ABC, abstractmethod
from typing import Dict, List, Optional, Tuple


class GitPlatformAPI(ABC):
    """Abstract base class for git platform APIs"""
    
    @abstractmethod
    def get_registration_token(self) -> str:
        """Get registration token for runner"""
        pass
    
    @abstractmethod
    def get_removal_token(self) -> str:
        """Get removal token for runner"""
        pass
    
    @abstractmethod
    def list_runners(self) -> List[Dict]:
        """List all runners"""
        pass
    
    @abstractmethod
    def get_pending_jobs(self) -> List[Dict]:
        """Get pending jobs in queue"""
        pass
    
    @abstractmethod
    def get_runner_labels(self) -> List[str]:
        """Get required labels from platform"""
        pass


class GitHubPlatform(GitPlatformAPI):
    """GitHub platform implementation"""
    
    def __init__(self, repository: str, token: str, api_url: str, logger: logging.Logger):
        self.repository = repository
        self.token = token
        self.api_url = api_url
        self.logger = logger
        self.platform_name = "github"
    
    def _make_request(self, endpoint: str, method: str = 'GET', data: Optional[Dict] = None) -> Dict:
        """Make authenticated request to GitHub API"""
        url = f"{self.api_url}/repos/{self.repository}/{endpoint}"
        
        headers = {
            'Authorization': f'token {self.token}',
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
    
    def get_registration_token(self) -> str:
        response = self._make_request('actions/runners/registration-token', method='POST')
        return response.get('token')
    
    def get_removal_token(self) -> str:
        response = self._make_request('actions/runners/remove-token', method='POST')
        return response.get('token')
    
    def list_runners(self) -> List[Dict]:
        response = self._make_request('actions/runners')
        return response.get('runners', [])
    
    def get_pending_jobs(self) -> List[Dict]:
        """Get pending workflow runs"""
        queued = self._make_request('actions/runs?status=queued').get('workflow_runs', [])
        in_progress = self._make_request('actions/runs?status=in_progress').get('workflow_runs', [])
        return queued + in_progress
    
    def get_runner_labels(self) -> List[str]:
        return ['self-hosted', 'linux']


class GitLabPlatform(GitPlatformAPI):
    """GitLab platform implementation (works with self-hosted too)"""
    
    def __init__(self, project_id: str, token: str, api_url: str, logger: logging.Logger):
        self.project_id = project_id
        self.token = token
        self.api_url = api_url.rstrip('/')
        self.logger = logger
        self.platform_name = "gitlab"
    
    def _make_request(self, endpoint: str, method: str = 'GET', data: Optional[Dict] = None) -> Dict:
        """Make authenticated request to GitLab API"""
        url = f"{self.api_url}/api/v4/{endpoint}"
        
        headers = {
            'PRIVATE-TOKEN': self.token,
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
            self.logger.error(f"GitLab API error: {e.code} - {error_msg}")
            raise
    
    def get_registration_token(self) -> str:
        """Get runner registration token from GitLab"""
        # For GitLab, we typically use a pre-generated token from settings
        # This is different from GitHub's dynamic tokens
        # Token should be provided in config
        return self.token
    
    def get_removal_token(self) -> str:
        return self.token
    
    def list_runners(self) -> List[Dict]:
        response = self._make_request(f'projects/{self.project_id}/runners')
        return response if isinstance(response, list) else []
    
    def get_pending_jobs(self) -> List[Dict]:
        """Get pending pipeline jobs"""
        response = self._make_request(f'projects/{self.project_id}/jobs?scope[]=pending&scope[]=running')
        return response if isinstance(response, list) else []
    
    def get_runner_labels(self) -> List[str]:
        return ['docker', 'linux']


class GiteaPlatform(GitPlatformAPI):
    """Gitea platform implementation"""
    
    def __init__(self, repository: str, token: str, api_url: str, logger: logging.Logger):
        self.repository = repository  # format: owner/repo
        self.token = token
        self.api_url = api_url.rstrip('/')
        self.logger = logger
        self.platform_name = "gitea"
    
    def _make_request(self, endpoint: str, method: str = 'GET', data: Optional[Dict] = None) -> Dict:
        """Make authenticated request to Gitea API"""
        url = f"{self.api_url}/api/v1/{endpoint}"
        
        headers = {
            'Authorization': f'token {self.token}',
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
            self.logger.error(f"Gitea API error: {e.code} - {error_msg}")
            raise
    
    def get_registration_token(self) -> str:
        """Get runner registration token from Gitea"""
        # Gitea Actions uses a similar pattern to GitHub
        response = self._make_request(f'repos/{self.repository}/actions/runners/registration-token', method='POST')
        return response.get('token')
    
    def get_removal_token(self) -> str:
        return self.token
    
    def list_runners(self) -> List[Dict]:
        response = self._make_request(f'repos/{self.repository}/actions/runners')
        return response if isinstance(response, list) else []
    
    def get_pending_jobs(self) -> List[Dict]:
        """Get pending action runs"""
        # Gitea Actions API (similar to GitHub)
        try:
            response = self._make_request(f'repos/{self.repository}/actions/runs?status=pending')
            return response if isinstance(response, list) else []
        except:
            return []
    
    def get_runner_labels(self) -> List[str]:
        return ['self-hosted', 'linux']


class GitPlatformFactory:
    """Factory for creating git platform instances"""
    
    @staticmethod
    def create(platform: str, config: Dict, logger: logging.Logger) -> GitPlatformAPI:
        """
        Create git platform instance
        
        Args:
            platform: Platform name (github, gitlab, gitea)
            config: Platform configuration
            logger: Logger instance
            
        Returns:
            GitPlatformAPI instance
        """
        platform = platform.lower()
        
        if platform == 'github':
            return GitHubPlatform(
                repository=config.get('repository'),
                token=config.get('token'),
                api_url=config.get('api_url', 'https://api.github.com'),
                logger=logger
            )
        
        elif platform == 'gitlab':
            return GitLabPlatform(
                project_id=config.get('project_id'),
                token=config.get('token'),
                api_url=config.get('api_url', 'https://gitlab.com'),
                logger=logger
            )
        
        elif platform == 'gitea':
            return GiteaPlatform(
                repository=config.get('repository'),
                token=config.get('token'),
                api_url=config.get('api_url'),
                logger=logger
            )
        
        else:
            raise ValueError(f"Unsupported platform: {platform}")
