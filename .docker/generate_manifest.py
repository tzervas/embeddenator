#!/usr/bin/env python3
"""
Generate Docker manifest for multi-arch images.

Creates manifest list for embeddenator container images across
multiple architectures (amd64, arm64) and pushes to GHCR.

Usage:
    python generate_manifest.py --version v0.20.0
    python generate_manifest.py --version latest
"""

import argparse
import json
import subprocess
import sys
from datetime import datetime
from typing import List, Dict


class ManifestGenerator:
    """Docker manifest generator for multi-arch images."""
    
    ARCHITECTURES = ["amd64", "arm64"]
    REGISTRY = "ghcr.io"
    NAMESPACE = "tzervas"
    IMAGE_NAME = "embeddenator"
    
    def __init__(self, version: str, dry_run: bool = False):
        self.version = version
        self.dry_run = dry_run
        self.manifest_name = f"{self.REGISTRY}/{self.NAMESPACE}/{self.IMAGE_NAME}:{version}"
    
    def get_image_tags(self) -> List[str]:
        """Get list of architecture-specific image tags."""
        return [
            f"{self.REGISTRY}/{self.NAMESPACE}/{self.IMAGE_NAME}:{self.version}-{arch}"
            for arch in self.ARCHITECTURES
        ]
    
    def run_command(self, cmd: List[str]) -> bool:
        """Execute command with error handling."""
        print(f"→ {' '.join(cmd)}")
        
        if self.dry_run:
            print("  (dry-run, skipped)")
            return True
        
        try:
            result = subprocess.run(
                cmd,
                check=True,
                capture_output=True,
                text=True
            )
            if result.stdout:
                print(result.stdout)
            return True
        except subprocess.CalledProcessError as e:
            print(f"✗ Error: {e.stderr}", file=sys.stderr)
            return False
    
    def create_manifest(self) -> bool:
        """Create manifest list."""
        print(f"\n📦 Creating manifest: {self.manifest_name}")
        
        # Create manifest
        cmd = ["docker", "manifest", "create", self.manifest_name] + self.get_image_tags()
        if not self.run_command(cmd):
            return False
        
        # Annotate each architecture
        for arch in self.ARCHITECTURES:
            image_tag = f"{self.REGISTRY}/{self.NAMESPACE}/{self.IMAGE_NAME}:{self.version}-{arch}"
            
            print(f"\n🏷️  Annotating {arch}...")
            cmd = [
                "docker", "manifest", "annotate",
                self.manifest_name,
                image_tag,
                "--arch", arch
            ]
            if not self.run_command(cmd):
                return False
        
        return True
    
    def inspect_manifest(self) -> Dict:
        """Inspect manifest list."""
        print(f"\n🔍 Inspecting manifest: {self.manifest_name}")
        
        if self.dry_run:
            print("  (dry-run, skipped)")
            return {}
        
        try:
            result = subprocess.run(
                ["docker", "manifest", "inspect", self.manifest_name],
                check=True,
                capture_output=True,
                text=True
            )
            manifest = json.loads(result.stdout)
            
            print(f"  Schema version: {manifest.get('schemaVersion')}")
            print(f"  Media type: {manifest.get('mediaType')}")
            print(f"  Manifests: {len(manifest.get('manifests', []))}")
            
            for m in manifest.get('manifests', []):
                platform = m.get('platform', {})
                print(f"    - {platform.get('architecture')}/{platform.get('os')}")
            
            return manifest
        except subprocess.CalledProcessError as e:
            print(f"✗ Error: {e.stderr}", file=sys.stderr)
            return {}
    
    def push_manifest(self) -> bool:
        """Push manifest list to registry."""
        print(f"\n🚀 Pushing manifest: {self.manifest_name}")
        
        cmd = ["docker", "manifest", "push", self.manifest_name]
        return self.run_command(cmd)
    
    def generate(self) -> bool:
        """Generate and push manifest."""
        print("=" * 70)
        print(f"Docker Manifest Generator")
        print(f"Version: {self.version}")
        print(f"Registry: {self.REGISTRY}/{self.NAMESPACE}/{self.IMAGE_NAME}")
        print(f"Architectures: {', '.join(self.ARCHITECTURES)}")
        print(f"Dry run: {self.dry_run}")
        print("=" * 70)
        
        # Step 1: Create manifest
        if not self.create_manifest():
            print("\n✗ Failed to create manifest", file=sys.stderr)
            return False
        
        # Step 2: Inspect manifest
        self.inspect_manifest()
        
        # Step 3: Push manifest
        if not self.push_manifest():
            print("\n✗ Failed to push manifest", file=sys.stderr)
            return False
        
        print(f"\n✓ Manifest successfully generated and pushed!")
        print(f"  Pull with: docker pull {self.manifest_name}")
        
        return True


def main():
    parser = argparse.ArgumentParser(
        description="Generate Docker manifest for multi-arch images"
    )
    parser.add_argument(
        "--version",
        required=True,
        help="Image version tag (e.g., v0.20.0, latest)"
    )
    parser.add_argument(
        "--dry-run",
        action="store_true",
        help="Print commands without executing"
    )
    
    args = parser.parse_args()
    
    generator = ManifestGenerator(args.version, args.dry_run)
    success = generator.generate()
    
    sys.exit(0 if success else 1)


if __name__ == "__main__":
    main()
