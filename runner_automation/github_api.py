"""
GitHub API Module

Handles communication with GitHub API for runner management.
"""

import json
import logging
import urllib.request
import urllib.error
from typing import Dict, List, Optional


class GitHubAPI:
    """GitHub API client for runner management"""
    
    def __init__(self, config, logger: logging.Logger):
        """
        Initialize GitHub API client
        
        Args:
            config: RunnerConfig instance
            logger: Logger instance
        """
        self.config = config
        self.logger = logger
    
    def _make_request(self, endpoint: str, method: str = 'GET', data: Optional[Dict] = None) -> Dict:
        """
        Make an authenticated request to GitHub API
        
        Args:
            endpoint: API endpoint (e.g., 'actions/runners/registration-token')
            method: HTTP method (GET, POST, etc.)
            data: Optional request data
            
        Returns:
            Response data as dictionary
            
        Raises:
            urllib.error.HTTPError: On HTTP errors
            Exception: On other errors
        """
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
            with urllib.request.urlopen(req, timeout=self.config.api_timeout) as response:
                return json.loads(response.read().decode('utf-8'))
        except urllib.error.HTTPError as e:
            error_msg = e.read().decode('utf-8') if e.fp else str(e)
            self.logger.error(f"GitHub API error: {e.code} - {error_msg}")
            raise
        except Exception as e:
            self.logger.error(f"Request failed: {e}")
            raise
    
    def get_registration_token(self) -> str:
        """
        Get a short-lived registration token from GitHub
        
        Returns:
            Registration token string
        """
        self.logger.info("Obtaining registration token from GitHub...")
        response = self._make_request('actions/runners/registration-token', method='POST')
        token = response.get('token')
        expires_at = response.get('expires_at')
        
        self.logger.info(f"Registration token obtained (expires: {expires_at})")
        return token
    
    def get_removal_token(self) -> str:
        """
        Get a removal token from GitHub
        
        Returns:
            Removal token string
        """
        self.logger.info("Obtaining removal token from GitHub...")
        response = self._make_request('actions/runners/remove-token', method='POST')
        return response.get('token')
    
    def list_runners(self) -> List[Dict]:
        """
        List all runners for the repository
        
        Returns:
            List of runner dictionaries
        """
        response = self._make_request('actions/runners')
        return response.get('runners', [])
    
    def get_runner_by_name(self, name: str) -> Optional[Dict]:
        """
        Find a runner by name
        
        Args:
            name: Runner name to search for
            
        Returns:
            Runner dictionary if found, None otherwise
        """
        runners = self.list_runners()
        for runner in runners:
            if runner.get('name') == name:
                return runner
        return None
    
    def get_workflow_runs(self, status: str = 'queued') -> List[Dict]:
        """
        Get workflow runs with specified status
        
        Args:
            status: Run status to filter by (queued, in_progress, etc.)
            
        Returns:
            List of workflow run dictionaries
        """
        try:
            response = self._make_request(f'actions/runs?status={status}')
            return response.get('workflow_runs', [])
        except Exception as e:
            self.logger.warning(f"Failed to get workflow runs: {e}")
            return []
    
    def count_queued_jobs(self) -> int:
        """
        Count jobs currently in queue
        
        Returns:
            Number of queued or in-progress jobs
        """
        queued_runs = self.get_workflow_runs('queued')
        in_progress_runs = self.get_workflow_runs('in_progress')
        return len(queued_runs) + len(in_progress_runs)
