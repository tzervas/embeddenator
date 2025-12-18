# Runner Automation Package

Complete lifecycle automation for GitHub Actions self-hosted runners.

## Package Structure

```
runner_automation/
├── __init__.py          # Package initialization and exports
├── config.py            # RunnerConfig - Configuration management
├── github_api.py        # GitHubAPI - GitHub API client
├── installer.py         # RunnerInstaller - Runner installation
├── runner.py            # Runner - Individual runner lifecycle
├── manager.py           # RunnerManager - Multi-runner orchestration
├── cli.py               # CLI - Command-line interface
└── README.md            # This file
```

## Module Overview

### config.py
**RunnerConfig** - Configuration management
- Loads settings from .env files and environment variables
- Validates configuration
- Detects system architecture
- Manages 50+ configuration options

### github_api.py
**GitHubAPI** - GitHub API client
- Obtains short-lived registration/removal tokens
- Lists and queries runners
- Monitors workflow runs and job queues
- Handles GitHub API authentication

### installer.py
**RunnerInstaller** - Runner installation
- Downloads GitHub Actions runner software
- Extracts and installs runners
- Detects latest runner version
- Supports x64 and ARM64 architectures

### runner.py
**Runner** - Individual runner lifecycle
- Registers runner with GitHub
- Starts and stops runner processes
- Deregisters runner from GitHub
- Cleans up installation
- Reports runner status

### manager.py
**RunnerManager** - Multi-runner orchestration
- Manages multiple runners
- Handles auto/manual deployment modes
- Monitors lifecycle and job queues
- Orchestrates registration/deregistration
- Cleans up Docker resources
- Provides status reporting

### cli.py
**CLI** - Command-line interface
- Argument parsing
- Command routing (register, start, stop, status, monitor, run)
- Configuration overrides
- Error handling

## Usage as a Package

### Direct Import

```python
from runner_automation import RunnerConfig, GitHubAPI, RunnerManager

# Load configuration
config = RunnerConfig()

# Create GitHub API client
github_api = GitHubAPI(config, None)

# Create and use manager
manager = RunnerManager(config, github_api)
manager.register_runners()
manager.start_runners()
manager.monitor_lifecycle()
```

### As a CLI Tool

```bash
# Via entry point script
python3 runner_manager.py run

# Direct module execution
python3 -m runner_automation.cli run
```

## Testing

```python
# Test individual modules
import runner_automation

# Test configuration
from runner_automation import RunnerConfig
config = RunnerConfig()
print(config.validate())

# Test GitHub API
from runner_automation import GitHubAPI
api = GitHubAPI(config, logger)
runners = api.list_runners()
```

## Future: Standalone Repository

This package is designed to be extracted into its own repository. The modular structure makes this straightforward:

1. Copy `runner_automation/` directory to new repo
2. Add `setup.py` or `pyproject.toml` for packaging
3. Publish to PyPI
4. Install via `pip install runner-automation`

The package has minimal dependencies (stdlib only) and is self-contained.

## Dependencies

- Python 3.7+
- Standard library only (no external dependencies)
- tar (system command for extraction)
- docker (optional, for cleanup feature)

## Documentation

See the main documentation at: `../docs/RUNNER_AUTOMATION.md`

## Version

Current version: 1.0.0
