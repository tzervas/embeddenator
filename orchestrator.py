#!/usr/bin/env python3
"""
Embeddenator Orchestrator
Unified build, test, package, and launch system for holographic computing substrate
"""

import argparse
import json
import os
import subprocess
import sys
import shutil
from pathlib import Path


class Orchestrator:
    def __init__(self, verbose=False, platform="linux/amd64"):
        self.verbose = verbose
        self.platform = platform
        self.root = Path(__file__).parent.absolute()
        self.workspace = self.root / "workspace"
        self.input_ws = self.root / "input_ws"
        
    def log(self, msg):
        if self.verbose:
            print(f"[orchestrator] {msg}")
            
    def run_cmd(self, cmd, check=True, capture=False):
        self.log(f"Running: {' '.join(cmd)}")
        if capture:
            result = subprocess.run(cmd, capture_output=True, text=True, check=check)
            return result.stdout
        else:
            result = subprocess.run(cmd, check=check)
            return result.returncode == 0
            
    def build_tool(self):
        """Build the embeddenator tool Docker image"""
        self.log("Building embeddenator tool image...")
        
        # Generate Cargo.lock if not exists
        if not (self.root / "Cargo.lock").exists():
            self.run_cmd(["cargo", "generate-lockfile"], check=True)
        
        # Build Docker image
        cmd = [
            "docker", "build",
            "-f", "Dockerfile.tool",
            "-t", "embeddenator-tool:latest",
            "--platform", self.platform,
            "."
        ]
        return self.run_cmd(cmd)
        
    def build_binary(self):
        """Build the embeddenator binary directly with cargo"""
        self.log("Building embeddenator binary...")
        cmd = ["cargo", "build", "--release"]
        return self.run_cmd(cmd)
        
    def test_basic(self):
        """Run basic cargo tests"""
        self.log("Running cargo tests...")
        cmd = ["cargo", "test", "--verbose"]
        return self.run_cmd(cmd)
        
    def test_integration(self):
        """Run integration test: ingest -> extract -> validate"""
        self.log("Running integration test...")
        
        # Ensure workspace exists
        self.workspace.mkdir(exist_ok=True)
        
        # Create test input if doesn't exist
        if not self.input_ws.exists():
            self.log("Creating test input workspace...")
            self.input_ws.mkdir(exist_ok=True)
            
            # Create test files
            (self.input_ws / "test.txt").write_text("Hello, holographic world!\n")
            (self.input_ws / "data.json").write_text(json.dumps({"test": True, "value": 42}))
            
            # Create a small binary file
            with open(self.input_ws / "binary.bin", "wb") as f:
                f.write(bytes(range(256)))
                
        # Run ingest
        self.log("Ingesting test data...")
        cmd = [
            "cargo", "run", "--release", "--",
            "ingest",
            "-i", str(self.input_ws),
            "-e", str(self.workspace / "root.engram"),
            "-m", str(self.workspace / "manifest.json"),
            "-v"
        ]
        if not self.run_cmd(cmd):
            return False
            
        # Check outputs exist
        if not (self.workspace / "root.engram").exists():
            self.log("ERROR: root.engram not created")
            return False
        if not (self.workspace / "manifest.json").exists():
            self.log("ERROR: manifest.json not created")
            return False
            
        # Run extract
        extract_dir = self.workspace / "extracted"
        extract_dir.mkdir(exist_ok=True)
        
        self.log("Extracting from engram...")
        cmd = [
            "cargo", "run", "--release", "--",
            "extract",
            "-e", str(self.workspace / "root.engram"),
            "-m", str(self.workspace / "manifest.json"),
            "-o", str(extract_dir),
            "-v"
        ]
        if not self.run_cmd(cmd):
            return False
            
        # Validate reconstruction
        self.log("Validating reconstruction...")
        
        # Check text file
        original_text = (self.input_ws / "test.txt").read_text()
        extracted_text = (extract_dir / "test.txt").read_text()
        if original_text != extracted_text:
            self.log(f"ERROR: Text file mismatch")
            self.log(f"  Original: {repr(original_text)}")
            self.log(f"  Extracted: {repr(extracted_text)}")
            return False
            
        # Check binary file
        original_bin = (self.input_ws / "binary.bin").read_bytes()
        extracted_bin = (extract_dir / "binary.bin").read_bytes()
        if original_bin != extracted_bin:
            self.log(f"ERROR: Binary file mismatch")
            return False
            
        self.log("✓ Integration test passed!")
        return True
        
    def test_full(self):
        """Run full test suite"""
        success = True
        success &= self.test_basic()
        success &= self.test_integration()
        return success
        
    def package(self):
        """Package the tool into a Docker image"""
        return self.build_tool()
        
    def clean(self):
        """Clean build artifacts and workspace"""
        self.log("Cleaning build artifacts...")
        
        if self.workspace.exists():
            shutil.rmtree(self.workspace)
            self.log(f"Removed {self.workspace}")
            
        # Clean cargo artifacts
        if (self.root / "target").exists():
            shutil.rmtree(self.root / "target")
            self.log("Removed target/")
            
        return True
        
    def info(self):
        """Display system information"""
        print("Embeddenator Orchestrator")
        print("=" * 50)
        print(f"Root: {self.root}")
        print(f"Workspace: {self.workspace}")
        print(f"Input: {self.input_ws}")
        print(f"Platform: {self.platform}")
        print()
        
        # Check for files
        if (self.workspace / "root.engram").exists():
            size = (self.workspace / "root.engram").stat().st_size
            print(f"Engram: {size:,} bytes")
            
        if (self.workspace / "manifest.json").exists():
            manifest = json.loads((self.workspace / "manifest.json").read_text())
            print(f"Files: {len(manifest['files'])}")
            print(f"Chunks: {manifest['total_chunks']}")
            
        return True


def main():
    parser = argparse.ArgumentParser(
        description="Embeddenator orchestrator for build, test, and deployment"
    )
    parser.add_argument(
        "--mode",
        choices=["build", "test", "full", "package", "clean", "info"],
        default="info",
        help="Operation mode"
    )
    parser.add_argument(
        "--platform",
        default="linux/amd64",
        help="Docker platform (default: linux/amd64)"
    )
    parser.add_argument(
        "-v", "--verbose",
        action="store_true",
        help="Verbose output"
    )
    parser.add_argument(
        "-i", "--interactive",
        action="store_true",
        help="Interactive mode (keep running)"
    )
    
    args = parser.parse_args()
    
    orch = Orchestrator(verbose=args.verbose, platform=args.platform)
    
    success = False
    
    if args.mode == "build":
        success = orch.build_binary()
    elif args.mode == "test":
        success = orch.test_full()
    elif args.mode == "full":
        success = orch.build_binary() and orch.test_full() and orch.package()
    elif args.mode == "package":
        success = orch.package()
    elif args.mode == "clean":
        success = orch.clean()
    elif args.mode == "info":
        success = orch.info()
        
    if not success:
        print("❌ Operation failed", file=sys.stderr)
        sys.exit(1)
    else:
        print("✅ Operation completed successfully")
        
    if args.interactive:
        print("\nInteractive mode - press Ctrl+C to exit")
        try:
            import time
            while True:
                time.sleep(1)
        except KeyboardInterrupt:
            print("\nExiting...")


if __name__ == "__main__":
    main()
