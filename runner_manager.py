#!/usr/bin/env python3
"""
GitHub Actions Self-Hosted Runner Manager

Entry point script for the runner automation system.
This is a thin wrapper around the runner_automation package.

For detailed documentation, see: docs/RUNNER_AUTOMATION.md
"""

import sys
from runner_automation.cli import main

if __name__ == '__main__':
    sys.exit(main())
