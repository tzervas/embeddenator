"""
Emulation Module

Handles QEMU emulation setup for cross-architecture runner support.
Enables ARM64 and RISC-V runners on x86_64 hardware.
Supports multiple container runtimes: Docker, Podman, or standalone QEMU.
"""

import logging
import subprocess
from typing import List, Optional, Tuple


class ContainerRuntime:
    """Container runtime detection and abstraction"""
    
    SUPPORTED_RUNTIMES = ['docker', 'podman']
    
    @staticmethod
    def detect_available_runtime() -> Optional[str]:
        """
        Detect which container runtime is available
        
        Returns:
            Runtime name ('docker' or 'podman') if available, None otherwise
        """
        for runtime in ContainerRuntime.SUPPORTED_RUNTIMES:
            try:
                result = subprocess.run(
                    ['which', runtime],
                    capture_output=True,
                    timeout=5
                )
                if result.returncode == 0:
                    return runtime
            except Exception:
                continue
        return None
    
    @staticmethod
    def get_runtime_command(runtime: str) -> str:
        """
        Get the command for the container runtime
        
        Args:
            runtime: Runtime name ('docker' or 'podman')
            
        Returns:
            Command string
        """
        if runtime in ContainerRuntime.SUPPORTED_RUNTIMES:
            return runtime
        return 'docker'  # Default fallback


class EmulationManager:
    """Manages QEMU emulation for cross-architecture support"""
    
    # Supported architectures and their emulation requirements
    SUPPORTED_ARCHITECTURES = {
        'x64': {
            'native': ['x86_64', 'amd64'],
            'emulation_required': False,
            'qemu_arch': None
        },
        'arm64': {
            'native': ['arm64', 'aarch64'],
            'emulation_required': True,
            'qemu_arch': 'aarch64',
            'qemu_package': 'qemu-user-static'
        },
        'riscv64': {
            'native': ['riscv64'],
            'emulation_required': True,
            'qemu_arch': 'riscv64',
            'qemu_package': 'qemu-user-static'
        }
    }
    
class EmulationManager:
    """Manages QEMU emulation for cross-architecture support"""
    
    # Supported architectures and their emulation requirements
    SUPPORTED_ARCHITECTURES = {
        'x64': {
            'native': ['x86_64', 'amd64'],
            'emulation_required': False,
            'qemu_arch': None
        },
        'arm64': {
            'native': ['arm64', 'aarch64'],
            'emulation_required': True,
            'qemu_arch': 'aarch64',
            'qemu_package': 'qemu-user-static'
        },
        'riscv64': {
            'native': ['riscv64'],
            'emulation_required': True,
            'qemu_arch': 'riscv64',
            'qemu_package': 'qemu-user-static'
        }
    }
    
    # Emulation methods
    EMULATION_METHODS = ['qemu', 'docker', 'podman']
    
    def __init__(self, logger: logging.Logger, emulation_method: Optional[str] = None):
        """
        Initialize emulation manager
        
        Args:
            logger: Logger instance
            emulation_method: Preferred emulation method ('qemu', 'docker', 'podman', or None for auto-detect)
        """
        self.logger = logger
        self._emulation_enabled = {}
        self.emulation_method = self._determine_emulation_method(emulation_method)
        self.container_runtime = None
        
        # If using container-based emulation, detect runtime
        if self.emulation_method in ('docker', 'podman'):
            self.container_runtime = self.emulation_method
        elif self.emulation_method == 'auto':
            # Try to detect container runtime
            self.container_runtime = ContainerRuntime.detect_available_runtime()
            if self.container_runtime:
                self.emulation_method = self.container_runtime
            else:
                self.emulation_method = 'qemu'
        
        self.logger.info(f"Emulation method: {self.emulation_method}")
        if self.container_runtime:
            self.logger.info(f"Container runtime: {self.container_runtime}")
    
    def _determine_emulation_method(self, preferred: Optional[str]) -> str:
        """
        Determine which emulation method to use
        
        Args:
            preferred: User's preferred method or None
            
        Returns:
            Emulation method to use
        """
        if preferred and preferred in self.EMULATION_METHODS:
            return preferred
        
        # Auto-detect: prefer container runtimes, fallback to QEMU
        runtime = ContainerRuntime.detect_available_runtime()
        if runtime:
            return runtime
        
        return 'qemu'
        """
        Check if QEMU user-static is installed
        
        Returns:
            True if installed, False otherwise
        """
        try:
            result = subprocess.run(
                ['which', 'qemu-aarch64-static'],
                capture_output=True,
                timeout=5
            )
            return result.returncode == 0
        except Exception as e:
            self.logger.debug(f"QEMU check failed: {e}")
            return False
    
    def check_binfmt_support(self, arch: str) -> bool:
        """
        Check if binfmt_misc has support for architecture
        
        Args:
            arch: Architecture to check (e.g., 'aarch64', 'riscv64')
            
        Returns:
            True if supported, False otherwise
        """
        try:
            result = subprocess.run(
                ['update-binfmts', '--display', f'qemu-{arch}'],
                capture_output=True,
                timeout=5
            )
            return result.returncode == 0
        except Exception:
            # Try alternative check via /proc
            try:
                binfmt_file = f'/proc/sys/fs/binfmt_misc/qemu-{arch}'
                with open(binfmt_file, 'r') as f:
                    content = f.read()
                    return 'enabled' in content
            except Exception:
                return False
    
    def install_qemu(self) -> bool:
        """
        Install QEMU user-static packages
        
        Returns:
            True if successful, False otherwise
        """
        self.logger.info("Installing QEMU user-static for emulation...")
        
        commands = [
            # Update package list
            ['apt-get', 'update'],
            # Install QEMU
            ['apt-get', 'install', '-y', 'qemu-user-static', 'binfmt-support']
        ]
        
        for cmd in commands:
            try:
                result = subprocess.run(
                    ['sudo'] + cmd,
                    capture_output=True,
                    text=True,
                    timeout=300
                )
                if result.returncode != 0:
                    self.logger.error(f"Command failed: {' '.join(cmd)}")
                    self.logger.error(f"Error: {result.stderr}")
                    return False
            except Exception as e:
                self.logger.error(f"Failed to install QEMU: {e}")
                return False
        
        self.logger.info("QEMU installation complete")
        return True
    
    def enable_binfmt(self, arch: str) -> bool:
        """
        Enable binfmt_misc support for architecture
        
        Args:
            arch: QEMU architecture (e.g., 'aarch64', 'riscv64')
            
        Returns:
            True if successful, False otherwise
        """
        self.logger.info(f"Enabling binfmt_misc for {arch} using {self.emulation_method}...")
        
        # Method 1: Try container-based setup (works best)
        if self.emulation_method in ('docker', 'podman') and self.container_runtime:
            return self._enable_binfmt_container(arch)
        
        # Method 2: Try native QEMU setup
        return self._enable_binfmt_native(arch)
    
    def _enable_binfmt_container(self, arch: str) -> bool:
        """
        Enable binfmt using container runtime
        
        Args:
            arch: QEMU architecture
            
        Returns:
            True if successful, False otherwise
        """
        runtime = self.container_runtime
        self.logger.info(f"Using {runtime} for binfmt setup...")
        
        try:
            # Use multiarch/qemu-user-static image for setup
            cmd = [
                runtime, 'run', '--rm', '--privileged',
                'multiarch/qemu-user-static', '--reset', '-p', 'yes'
            ]
            
            result = subprocess.run(
                cmd,
                capture_output=True,
                text=True,
                timeout=120
            )
            
            if result.returncode == 0:
                self.logger.info(f"Container-based binfmt setup successful for {arch}")
                return True
            else:
                self.logger.error(f"Container setup failed: {result.stderr}")
                return False
                
        except Exception as e:
            self.logger.error(f"Failed to enable binfmt via {runtime}: {e}")
            return False
    
    def _enable_binfmt_native(self, arch: str) -> bool:
        """
        Enable binfmt using native QEMU
        
        Args:
            arch: QEMU architecture
            
        Returns:
            True if successful, False otherwise
        """
        self.logger.info(f"Using native QEMU for binfmt setup...")
        
        try:
            # Try to enable via update-binfmts
            result = subprocess.run(
                ['sudo', 'update-binfmts', '--enable', f'qemu-{arch}'],
                capture_output=True,
                timeout=10
            )
            
            if result.returncode == 0:
                self.logger.info(f"Native binfmt setup successful for {arch}")
                return True
            
            return False
            
        except Exception as e:
            self.logger.error(f"Failed to enable binfmt natively: {e}")
            return False
    
    def setup_emulation(self, target_arch: str, host_arch: str) -> bool:
        """
        Setup emulation for target architecture on host architecture
        
        Args:
            target_arch: Target architecture (x64, arm64, riscv64)
            host_arch: Host architecture (x64, arm64, riscv64)
            
        Returns:
            True if emulation setup successful or not needed, False otherwise
        """
        # Check if emulation is needed
        if not self.is_emulation_needed(target_arch, host_arch):
            self.logger.info(f"No emulation needed: {target_arch} on {host_arch}")
            return True
        
        arch_info = self.SUPPORTED_ARCHITECTURES.get(target_arch)
        if not arch_info:
            self.logger.error(f"Unsupported architecture: {target_arch}")
            return False
        
        qemu_arch = arch_info['qemu_arch']
        self.logger.info(f"Setting up emulation for {target_arch} (QEMU: {qemu_arch})")
        
        # Check if already enabled
        if self.check_binfmt_support(qemu_arch):
            self.logger.info(f"Emulation already enabled for {qemu_arch}")
            self._emulation_enabled[target_arch] = True
            return True
        
        # Check if QEMU is installed
        if not self.check_qemu_installed():
            self.logger.warning("QEMU not installed, attempting installation...")
            if not self.install_qemu():
                self.logger.error("Failed to install QEMU")
                return False
        
        # Enable binfmt
        if not self.enable_binfmt(qemu_arch):
            self.logger.error(f"Failed to enable emulation for {qemu_arch}")
            return False
        
        # Verify it works
        if not self.verify_emulation(target_arch):
            self.logger.error(f"Emulation verification failed for {target_arch}")
            return False
        
        self._emulation_enabled[target_arch] = True
        self.logger.info(f"Emulation successfully enabled for {target_arch}")
        return True
    
    def is_emulation_needed(self, target_arch: str, host_arch: str) -> bool:
        """
        Check if emulation is needed for target arch on host arch
        
        Args:
            target_arch: Target architecture
            host_arch: Host architecture
            
        Returns:
            True if emulation needed, False otherwise
        """
        if target_arch == host_arch:
            return False
        
        # x64 and amd64 are the same
        if target_arch in ('x64', 'amd64') and host_arch in ('x64', 'amd64'):
            return False
        
        # ARM64 and aarch64 are the same
        if target_arch in ('arm64', 'aarch64') and host_arch in ('arm64', 'aarch64'):
            return False
        
        return True
    
    def verify_emulation(self, arch: str) -> bool:
        """
        Verify emulation works for architecture
        
        Args:
            arch: Architecture to verify (x64, arm64, riscv64)
            
        Returns:
            True if emulation works, False otherwise
        """
        # Map our arch names to platform names
        platform_map = {
            'x64': 'linux/amd64',
            'arm64': 'linux/arm64',
            'riscv64': 'linux/riscv64'
        }
        
        platform = platform_map.get(arch)
        if not platform:
            return False
        
        # Determine which container runtime to use for verification
        runtime = self.container_runtime if self.container_runtime else 'docker'
        
        # Check if runtime is available
        if not self._is_runtime_available(runtime):
            self.logger.warning(f"{runtime} not available, skipping verification")
            # If no container runtime, assume QEMU is working if binfmt is enabled
            arch_info = self.SUPPORTED_ARCHITECTURES.get(arch)
            if arch_info and arch_info['qemu_arch']:
                return self.check_binfmt_support(arch_info['qemu_arch'])
            return True
        
        try:
            self.logger.info(f"Verifying emulation for {arch} ({platform}) using {runtime}...")
            
            # Try to run a simple command in the target architecture
            result = subprocess.run(
                [runtime, 'run', '--rm', '--platform', platform,
                 'alpine:latest', 'uname', '-m'],
                capture_output=True,
                text=True,
                timeout=90
            )
            
            if result.returncode == 0:
                detected_arch = result.stdout.strip()
                self.logger.info(f"Emulation test successful: detected {detected_arch}")
                return True
            else:
                self.logger.error(f"Emulation test failed: {result.stderr}")
                return False
                
        except Exception as e:
            self.logger.error(f"Emulation verification failed: {e}")
            return False
    
    def _is_runtime_available(self, runtime: str) -> bool:
        """
        Check if container runtime is available
        
        Args:
            runtime: Runtime name ('docker' or 'podman')
            
        Returns:
            True if available, False otherwise
        """
        try:
            result = subprocess.run(
                ['which', runtime],
                capture_output=True,
                timeout=5
            )
            return result.returncode == 0
        except Exception:
            return False
    
    def get_supported_architectures(self) -> List[str]:
        """
        Get list of supported architectures
        
        Returns:
            List of architecture names
        """
        return list(self.SUPPORTED_ARCHITECTURES.keys())
    
    def is_emulation_enabled(self, arch: str) -> bool:
        """
        Check if emulation is enabled for architecture
        
        Args:
            arch: Architecture to check
            
        Returns:
            True if enabled, False otherwise
        """
        return self._emulation_enabled.get(arch, False)
