#!/usr/bin/env python3
"""
CLI Module

Command-line interface for runner automation system.
"""

import argparse
import sys

from .config import RunnerConfig
from .github_api import GitHubAPI
from .manager import RunnerManager


def main():
    """Main entry point for CLI"""
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
    
    # Create GitHub API client
    github_api = GitHubAPI(config, None)  # Logger will be set by manager
    
    # Create manager
    manager = RunnerManager(config, github_api)
    
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
