#!/usr/bin/env python3
"""
Holographic OS Container Builder
Generates fully holographic VSA-style OS containers for various Debian/Ubuntu releases
Supports configuration file and CLI parameter overrides
"""

import argparse
import json
import os
import subprocess
import sys
import time
import yaml
from pathlib import Path
from typing import Dict, List, Tuple, Optional


class HolographicOSBuilder:
    def __init__(self, verbose=False, registry="ghcr.io", repo="tzervas/embeddenator", 
                 config_file="os_config.yaml", tag_suffix=""):
        self.verbose = verbose
        self.registry = registry
        self.repo = repo
        self.tag_suffix = tag_suffix
        self.root = Path(__file__).parent.absolute()
        self.workspace = self.root / "os_workspace"
        self.build_dir = self.root / "os_builds"
        
        # Load OS configurations from file or use defaults
        self.os_configs = self.load_config(config_file)
            "debian-stable-amd64": {
                "base_image": "debian:stable",
                "platform": "linux/amd64",
                "description": "Debian Stable (amd64)",
                "test_commands": [
                    "cat /etc/os-release",
                    "ls -la /bin /usr/bin",
                    "dpkg --version",
                    "apt --version",
                ]
            },
            "debian-stable-arm64": {
                "base_image": "debian:stable",
                "platform": "linux/arm64",
                "description": "Debian Stable (arm64)",
                "test_commands": [
                    "cat /etc/os-release",
                    "ls -la /bin /usr/bin",
                    "dpkg --version",
                    "apt --version",
                ]
            },
            "debian-testing-amd64": {
                "base_image": "debian:testing",
                "platform": "linux/amd64",
                "description": "Debian Testing (amd64)",
                "test_commands": [
                    "cat /etc/os-release",
                    "ls -la /bin /usr/bin",
                    "dpkg --version",
                    "apt --version",
                ]
            },
            "debian-testing-arm64": {
                "base_image": "debian:testing",
                "platform": "linux/arm64",
                "description": "Debian Testing (arm64)",
                "test_commands": [
                    "cat /etc/os-release",
                    "ls -la /bin /usr/bin",
                    "dpkg --version",
                    "apt --version",
                ]
            },
            "ubuntu-stable-amd64": {
                "base_image": "ubuntu:latest",
                "platform": "linux/amd64",
                "description": "Ubuntu Stable (amd64)",
                "test_commands": [
                    "cat /etc/os-release",
                    "ls -la /bin /usr/bin",
                    "dpkg --version",
                    "apt --version",
                ]
            },
            "ubuntu-stable-arm64": {
                "base_image": "ubuntu:latest",
                "platform": "linux/arm64",
                "description": "Ubuntu Stable (arm64)",
                "test_commands": [
                    "cat /etc/os-release",
                    "ls -la /bin /usr/bin",
                    "dpkg --version",
                    "apt --version",
                ]
            },
            "ubuntu-testing-amd64": {
                "base_image": "ubuntu:devel",
                "platform": "linux/amd64",
                "description": "Ubuntu Testing/Devel (amd64)",
                "test_commands": [
                    "cat /etc/os-release",
                    "ls -la /bin /usr/bin",
                    "dpkg --version",
                    "apt --version",
                ]
            },
            "ubuntu-testing-arm64": {
                "base_image": "ubuntu:devel",
                "platform": "linux/arm64",
                "description": "Ubuntu Testing/Devel (arm64)",
                "test_commands": [
                    "cat /etc/os-release",
                    "ls -la /bin /usr/bin",
                    "dpkg --version",
                    "apt --version",
                ]
            },
        
    def load_config(self, config_file: str) -> Dict:
        """Load OS configurations from YAML file or use defaults"""
        config_path = self.root / config_file
        
        if config_path.exists():
            try:
                with open(config_path, 'r') as f:
                    config_data = yaml.safe_load(f)
                    
                # Extract OS configs and add test commands
                os_configs = {}
                test_commands = config_data.get('test_commands', [
                    "cat /etc/os-release",
                    "ls -la /bin /usr/bin",
                    "dpkg --version",
                    "apt --version",
                ])
                
                for name, cfg in config_data.get('os_configs', {}).items():
                    if cfg.get('enabled', True):
                        os_configs[name] = {
                            "base_image": cfg['base_image'],
                            "platform": cfg['platform'],
                            "description": cfg['description'],
                            "test_commands": test_commands
                        }
                
                self.log(f"Loaded {len(os_configs)} OS configs from {config_file}")
                return os_configs
            except Exception as e:
                self.log(f"Warning: Could not load config file: {e}, using defaults")
        
        # Default configurations if file not found
        return self._get_default_configs()
    
    def _get_default_configs(self) -> Dict:
        """Return default OS configurations"""
        return {
            "debian-stable-amd64": {
                "base_image": "debian:stable",
                "platform": "linux/amd64",
                "description": "Debian Stable (amd64)",
                "test_commands": ["cat /etc/os-release", "ls -la /bin", "dpkg --version", "apt --version"]
            },
            "debian-stable-arm64": {
                "base_image": "debian:stable",
                "platform": "linux/arm64",
                "description": "Debian Stable (arm64)",
                "test_commands": ["cat /etc/os-release", "ls -la /bin", "dpkg --version", "apt --version"]
            },
            "debian-testing-amd64": {
                "base_image": "debian:testing",
                "platform": "linux/amd64",
                "description": "Debian Testing (amd64)",
                "test_commands": ["cat /etc/os-release", "ls -la /bin", "dpkg --version", "apt --version"]
            },
            "debian-testing-arm64": {
                "base_image": "debian:testing",
                "platform": "linux/arm64",
                "description": "Debian Testing (arm64)",
                "test_commands": ["cat /etc/os-release", "ls -la /bin", "dpkg --version", "apt --version"]
            },
            "ubuntu-stable-amd64": {
                "base_image": "ubuntu:latest",
                "platform": "linux/amd64",
                "description": "Ubuntu Stable (amd64)",
                "test_commands": ["cat /etc/os-release", "ls -la /bin", "dpkg --version", "apt --version"]
            },
            "ubuntu-stable-arm64": {
                "base_image": "ubuntu:latest",
                "platform": "linux/arm64",
                "description": "Ubuntu Stable (arm64)",
                "test_commands": ["cat /etc/os-release", "ls -la /bin", "dpkg --version", "apt --version"]
            },
            "ubuntu-testing-amd64": {
                "base_image": "ubuntu:devel",
                "platform": "linux/amd64",
                "description": "Ubuntu Testing/Devel (amd64)",
                "test_commands": ["cat /etc/os-release", "ls -la /bin", "dpkg --version", "apt --version"]
            },
            "ubuntu-testing-arm64": {
                "base_image": "ubuntu:devel",
                "platform": "linux/arm64",
                "description": "Ubuntu Testing/Devel (arm64)",
                "test_commands": ["cat /etc/os-release", "ls -la /bin", "dpkg --version", "apt --version"]
            },
        }
        
    def log(self, msg):
        timestamp = time.strftime("%Y-%m-%d %H:%M:%S")
        if self.verbose:
            print(f"[{timestamp}] {msg}")
            
    def run_cmd(self, cmd, check=True, capture=False, cwd=None):
        self.log(f"Running: {' '.join(cmd)}")
        if capture:
            result = subprocess.run(cmd, capture_output=True, text=True, check=check, cwd=cwd)
            return result.stdout
        else:
            result = subprocess.run(cmd, check=check, cwd=cwd)
            return result.returncode == 0
            
    def setup_workspace(self):
        """Create necessary directories"""
        self.workspace.mkdir(exist_ok=True)
        self.build_dir.mkdir(exist_ok=True)
        self.log(f"Workspace: {self.workspace}")
        self.log(f"Build dir: {self.build_dir}")
        
    def build_embeddenator_tool(self):
        """Build the embeddenator tool first"""
        self.log("Building embeddenator tool...")
        cmd = ["cargo", "build", "--release"]
        if not self.run_cmd(cmd):
            raise RuntimeError("Failed to build embeddenator tool")
        return True
        
    def extract_base_os(self, config_name: str) -> Path:
        """Extract a base OS image to filesystem"""
        config = self.os_configs[config_name]
        extract_dir = self.workspace / f"{config_name}_rootfs"
        
        self.log(f"Extracting {config['description']}...")
        
        # Create extraction directory
        extract_dir.mkdir(exist_ok=True, parents=True)
        
        # Pull and export the base image
        self.log(f"Pulling {config['base_image']}...")
        cmd = ["docker", "pull", "--platform", config["platform"], config["base_image"]]
        if not self.run_cmd(cmd):
            raise RuntimeError(f"Failed to pull {config['base_image']}")
            
        # Create a container and export it
        self.log("Exporting container filesystem...")
        container_name = f"embeddenator-extract-{config_name}"
        
        # Remove old container if exists
        subprocess.run(["docker", "rm", "-f", container_name], 
                      capture_output=True, check=False)
        
        # Create container
        cmd = ["docker", "create", "--name", container_name, 
               "--platform", config["platform"], config["base_image"], "/bin/true"]
        if not self.run_cmd(cmd):
            raise RuntimeError("Failed to create container")
            
        # Export filesystem
        export_tar = self.workspace / f"{config_name}_export.tar"
        with open(export_tar, "wb") as f:
            result = subprocess.run(
                ["docker", "export", container_name],
                stdout=f,
                check=True
            )
            
        # Extract tar
        cmd = ["tar", "xf", str(export_tar), "-C", str(extract_dir)]
        if not self.run_cmd(cmd):
            raise RuntimeError("Failed to extract tar")
            
        # Cleanup
        subprocess.run(["docker", "rm", "-f", container_name], 
                      capture_output=True, check=False)
        export_tar.unlink()
        
        self.log(f"✓ Extracted to {extract_dir}")
        return extract_dir
        
    def ingest_to_engram(self, rootfs_path: Path, config_name: str) -> Tuple[Path, Path]:
        """Ingest a rootfs into engram format"""
        self.log(f"Ingesting {config_name} to engram...")
        
        engram_path = self.build_dir / f"{config_name}.engram"
        manifest_path = self.build_dir / f"{config_name}.manifest.json"
        
        # Build command
        embeddenator = self.root / "target" / "release" / "embeddenator"
        cmd = [
            str(embeddenator),
            "ingest",
            "-i", str(rootfs_path),
            "-e", str(engram_path),
            "-m", str(manifest_path),
            "-v"
        ]
        
        start_time = time.time()
        if not self.run_cmd(cmd):
            raise RuntimeError("Failed to ingest to engram")
        elapsed = time.time() - start_time
        
        # Get sizes
        original_size = sum(f.stat().st_size for f in rootfs_path.rglob('*') if f.is_file())
        engram_size = engram_path.stat().st_size
        manifest_size = manifest_path.stat().st_size
        
        self.log(f"✓ Ingestion complete in {elapsed:.2f}s")
        self.log(f"  Original: {original_size / 1024 / 1024:.2f} MB")
        self.log(f"  Engram: {engram_size / 1024 / 1024:.2f} MB")
        self.log(f"  Manifest: {manifest_size / 1024:.2f} KB")
        self.log(f"  Ratio: {engram_size / original_size * 100:.1f}%")
        
        return engram_path, manifest_path
        
    def extract_from_engram(self, engram_path: Path, manifest_path: Path, 
                           config_name: str) -> Path:
        """Extract OS from engram back to filesystem"""
        self.log(f"Extracting {config_name} from engram...")
        
        extract_dir = self.workspace / f"{config_name}_extracted"
        extract_dir.mkdir(exist_ok=True, parents=True)
        
        embeddenator = self.root / "target" / "release" / "embeddenator"
        cmd = [
            str(embeddenator),
            "extract",
            "-e", str(engram_path),
            "-m", str(manifest_path),
            "-o", str(extract_dir),
            "-v"
        ]
        
        start_time = time.time()
        if not self.run_cmd(cmd):
            raise RuntimeError("Failed to extract from engram")
        elapsed = time.time() - start_time
        
        self.log(f"✓ Extraction complete in {elapsed:.2f}s")
        return extract_dir
        
    def validate_reconstruction(self, original: Path, reconstructed: Path) -> bool:
        """Validate that reconstruction matches original"""
        self.log("Validating reconstruction...")
        
        # Compare directory structures
        original_files = sorted([f.relative_to(original) for f in original.rglob('*') if f.is_file()])
        reconstructed_files = sorted([f.relative_to(reconstructed) for f in reconstructed.rglob('*') if f.is_file()])
        
        self.log(f"  Original files: {len(original_files)}")
        self.log(f"  Reconstructed files: {len(reconstructed_files)}")
        
        if len(original_files) != len(reconstructed_files):
            self.log(f"  ❌ File count mismatch!")
            return False
            
        # Sample file comparison (check first 100 and some random ones)
        import random
        sample_size = min(100, len(original_files))
        sample_files = original_files[:50] + random.sample(original_files[50:], min(50, len(original_files) - 50))
        
        mismatches = 0
        for rel_path in sample_files:
            orig_file = original / rel_path
            recon_file = reconstructed / rel_path
            
            if not recon_file.exists():
                self.log(f"  ❌ Missing file: {rel_path}")
                mismatches += 1
                continue
                
            # Compare sizes
            if orig_file.stat().st_size != recon_file.stat().st_size:
                self.log(f"  ❌ Size mismatch: {rel_path}")
                mismatches += 1
                continue
                
            # Compare content for small files
            if orig_file.stat().st_size < 1024 * 1024:  # < 1MB
                try:
                    if orig_file.read_bytes() != recon_file.read_bytes():
                        self.log(f"  ❌ Content mismatch: {rel_path}")
                        mismatches += 1
                except Exception as e:
                    self.log(f"  ⚠️  Could not compare {rel_path}: {e}")
                    
        if mismatches > 0:
            self.log(f"  ❌ Found {mismatches} mismatches in sample")
            return False
            
        self.log(f"  ✓ Validation passed (sampled {sample_size} files)")
        return True
        
    def build_holographic_container(self, engram_path: Path, manifest_path: Path,
                                   config_name: str) -> str:
        """Build a Docker container from engram"""
        config = self.os_configs[config_name]
        version = self.get_version()
        image_name = f"embeddenator-holo-{config_name}:{version}{self.tag_suffix}"
        
        self.log(f"Building holographic container: {image_name}")
        
        # Create Dockerfile
        dockerfile_content = f"""FROM embeddenator-tool:latest AS extractor
WORKDIR /build
COPY {engram_path.name} /root.engram
COPY {manifest_path.name} /manifest.json
RUN embeddenator extract --output-dir /rootfs \\
    --engram /root.engram \\
    --manifest /manifest.json \\
    --verbose

FROM scratch
COPY --from=extractor /rootfs/ /
# Restore basic shell if available
CMD ["/bin/sh"]
"""
        
        dockerfile = self.build_dir / f"Dockerfile.{config_name}"
        dockerfile.write_text(dockerfile_content)
        
        # Build container
        cmd = [
            "docker", "build",
            "-f", str(dockerfile),
            "-t", image_name,
            "--platform", config["platform"],
            str(self.build_dir)
        ]
        
        if not self.run_cmd(cmd):
            raise RuntimeError(f"Failed to build {image_name}")
            
        self.log(f"✓ Built {image_name}")
        return image_name
        
    def test_holographic_container(self, image_name: str, config_name: str) -> bool:
        """Test a holographic container"""
        config = self.os_configs[config_name]
        self.log(f"Testing {image_name}...")
        
        success = True
        for test_cmd in config["test_commands"]:
            self.log(f"  Testing: {test_cmd}")
            cmd = [
                "docker", "run", "--rm",
                "--platform", config["platform"],
                image_name,
                "/bin/sh", "-c", test_cmd
            ]
            
            try:
                result = subprocess.run(cmd, capture_output=True, text=True, 
                                      timeout=30, check=False)
                if result.returncode == 0:
                    self.log(f"    ✓ Passed")
                else:
                    self.log(f"    ❌ Failed (exit code {result.returncode})")
                    if self.verbose and result.stderr:
                        self.log(f"    Error: {result.stderr[:200]}")
                    success = False
            except subprocess.TimeoutExpired:
                self.log(f"    ❌ Timeout")
                success = False
            except Exception as e:
                self.log(f"    ❌ Error: {e}")
                success = False
                
        return success
    
    def get_version(self) -> str:
        """Get version from Cargo.toml"""
        cargo_toml = self.root / "Cargo.toml"
        if cargo_toml.exists():
            with open(cargo_toml, 'r') as f:
                for line in f:
                    if line.startswith('version ='):
                        version = line.split('=')[1].strip().strip('"')
                        return version
        return "0.1.0"  # fallback
        
    def push_to_registry(self, image_name: str, config_name: str) -> bool:
        """Push image to container registry"""
        full_name = f"{self.registry}/{self.repo}/{image_name}"
        
        self.log(f"Tagging {image_name} as {full_name}")
        cmd = ["docker", "tag", image_name, full_name]
        if not self.run_cmd(cmd):
            return False
            
        self.log(f"Pushing {full_name}...")
        cmd = ["docker", "push", full_name]
        if not self.run_cmd(cmd):
            return False
            
        self.log(f"✓ Pushed {full_name}")
        return True
        
    def build_os(self, config_name: str, skip_push: bool = False) -> bool:
        """Complete workflow for one OS"""
        self.log(f"\n{'='*60}")
        self.log(f"Building Holographic OS: {config_name}")
        self.log(f"{'='*60}\n")
        
        try:
            # Extract base OS
            rootfs = self.extract_base_os(config_name)
            
            # Ingest to engram
            engram, manifest = self.ingest_to_engram(rootfs, config_name)
            
            # Extract and validate
            extracted = self.extract_from_engram(engram, manifest, config_name)
            if not self.validate_reconstruction(rootfs, extracted):
                raise RuntimeError("Validation failed!")
                
            # Build holographic container
            image_name = self.build_holographic_container(engram, manifest, config_name)
            
            # Test container
            if not self.test_holographic_container(image_name, config_name):
                raise RuntimeError("Container tests failed!")
                
            # Push to registry
            if not skip_push:
                if not self.push_to_registry(image_name, config_name):
                    self.log("⚠️  Push failed, but build succeeded")
            else:
                self.log("Skipping registry push (--skip-push)")
                
            self.log(f"\n✅ Successfully built holographic OS: {config_name}\n")
            return True
            
        except Exception as e:
            self.log(f"\n❌ Failed to build {config_name}: {e}\n")
            return False
            
    def build_all(self, skip_push: bool = False):
        """Build all OS configurations"""
        self.setup_workspace()
        
        # First build the embeddenator tool
        self.build_embeddenator_tool()
        
        # Build tool container
        self.log("Building embeddenator-tool container...")
        cmd = ["docker", "build", "-f", "Dockerfile.tool", "-t", "embeddenator-tool:latest", "."]
        if not self.run_cmd(cmd):
            raise RuntimeError("Failed to build embeddenator-tool")
            
        results = {}
        for config_name in self.os_configs.keys():
            results[config_name] = self.build_os(config_name, skip_push)
            
        # Summary
        self.log(f"\n{'='*60}")
        self.log("Build Summary")
        self.log(f"{'='*60}\n")
        
        for config_name, success in results.items():
            status = "✅ SUCCESS" if success else "❌ FAILED"
            self.log(f"  {config_name}: {status}")
            
        total = len(results)
        successful = sum(1 for s in results.values() if s)
        self.log(f"\nTotal: {successful}/{total} successful")
        
        return all(results.values())


def main():
    parser = argparse.ArgumentParser(
        description="Build holographic OS containers"
    )
    parser.add_argument(
        "--os",
        choices=["all", 
                 "debian-stable-amd64", "debian-stable-arm64",
                 "debian-testing-amd64", "debian-testing-arm64",
                 "ubuntu-stable-amd64", "ubuntu-stable-arm64",
                 "ubuntu-testing-amd64", "ubuntu-testing-arm64"],
        default="all",
        help="OS to build (default: all)"
    )
    parser.add_argument(
        "--registry",
        default="ghcr.io",
        help="Container registry (default: ghcr.io)"
    )
    parser.add_argument(
        "--repo",
        default="tzervas/embeddenator",
        help="Repository path (default: tzervas/embeddenator)"
    )
    parser.add_argument(
        "--skip-push",
        action="store_true",
        help="Skip pushing to registry"
    )
    parser.add_argument(
        "-v", "--verbose",
        action="store_true",
        help="Verbose output"
    )
    
    args = parser.parse_args()
    
    builder = HolographicOSBuilder(
        verbose=args.verbose,
        registry=args.registry,
        repo=args.repo
    )
    
    try:
        if args.os == "all":
            success = builder.build_all(skip_push=args.skip_push)
        else:
            builder.setup_workspace()
            builder.build_embeddenator_tool()
            
            # Build tool container
            builder.log("Building embeddenator-tool container...")
            cmd = ["docker", "build", "-f", "Dockerfile.tool", 
                   "-t", "embeddenator-tool:latest", "."]
            builder.run_cmd(cmd)
            
            success = builder.build_os(args.os, skip_push=args.skip_push)
            
        sys.exit(0 if success else 1)
        
    except KeyboardInterrupt:
        print("\n\nInterrupted by user")
        sys.exit(130)
    except Exception as e:
        print(f"\n❌ Fatal error: {e}", file=sys.stderr)
        sys.exit(1)


if __name__ == "__main__":
    main()
