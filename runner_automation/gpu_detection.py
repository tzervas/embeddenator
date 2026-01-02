"""
GPU Detection Module

Detects and manages GPU resources for runner deployment.
Supports Intel, AMD, and NVIDIA GPUs with tensor ops capabilities.
"""

import logging
import re
import subprocess
from typing import Dict, List, Optional, Tuple


class GPUInfo:
    """Information about a detected GPU"""
    
    def __init__(self, vendor: str, name: str, index: int, memory_mb: int = 0, 
                 compute_capability: str = "", pci_id: str = ""):
        self.vendor = vendor  # nvidia, amd, intel
        self.name = name
        self.index = index
        self.memory_mb = memory_mb
        self.compute_capability = compute_capability
        self.pci_id = pci_id
        self.is_inference_capable = False
        self.is_training_capable = False
    
    def __repr__(self):
        return f"GPU({self.vendor} {self.name}, {self.memory_mb}MB)"


class GPUDetector:
    """Detect and classify GPUs for runner deployment"""
    
    # Minimum compute capabilities for tensor ops
    NVIDIA_MIN_COMPUTE = 7.0  # Volta and newer (V100, T4, RTX 20xx+)
    
    # Known inference-optimized GPUs
    INFERENCE_GPUS = {
        'nvidia': ['T4', 'A10', 'A16', 'A2', 'L4', 'L40'],
        'intel': ['Flex', 'Max', 'Arc A'],
        'amd': [
            # Data Center / Professional
            'MI210', 'MI250', 'MI300', 'MI100', 'MI60', 'MI50',
            'Instinct',
            # Prosumer / High-end Consumer (RDNA2 and newer)
            'RX 6900', 'RX 6800', 'RX 6700',
            'RX 7900', 'RX 7800', 'RX 7700', 'RX 7600',
            # Professional Workstation
            'Radeon Pro W6800', 'Radeon Pro W6900', 'Radeon Pro W7900',
            'Radeon Pro V620', 'Radeon Pro V520',
            # Consumer RDNA3
            'RX 7950', 'RX 7800 XT', 'RX 7700 XT',
        ]
    }
    
    # Consumer AMD GPUs with ROCm support (capable but not optimized for inference)
    AMD_ROCM_CAPABLE = [
        'RX 6950', 'RX 6900', 'RX 6800', 'RX 6750', 'RX 6700', 'RX 6650', 'RX 6600',
        'RX 7900', 'RX 7800', 'RX 7700', 'RX 7600',
        'Vega 64', 'Vega 56', 'Vega VII',
        'Radeon VII'
    ]
    
    # Apple Silicon optimization
    APPLE_CHIPS = ['M1', 'M2', 'M3']
    
    def __init__(self, logger: logging.Logger):
        """
        Initialize GPU detector
        
        Args:
            logger: Logger instance
        """
        self.logger = logger
        self.gpus: List[GPUInfo] = []
    
    def detect_all_gpus(self) -> List[GPUInfo]:
        """
        Detect all available GPUs
        
        Returns:
            List of detected GPUs
        """
        self.gpus = []
        
        # Try NVIDIA GPUs first
        nvidia_gpus = self._detect_nvidia_gpus()
        self.gpus.extend(nvidia_gpus)
        
        # Try AMD GPUs
        amd_gpus = self._detect_amd_gpus()
        self.gpus.extend(amd_gpus)
        
        # Try Intel GPUs
        intel_gpus = self._detect_intel_gpus()
        self.gpus.extend(intel_gpus)
        
        # Check for Apple Silicon
        if self._is_apple_silicon():
            apple_gpu = self._detect_apple_gpu()
            if apple_gpu:
                self.gpus.append(apple_gpu)
        
        # Classify GPUs for inference/training
        for gpu in self.gpus:
            self._classify_gpu_capabilities(gpu)
        
        return self.gpus
    
    def _detect_nvidia_gpus(self) -> List[GPUInfo]:
        """Detect NVIDIA GPUs using nvidia-smi"""
        gpus = []
        
        try:
            # Check if nvidia-smi is available
            result = subprocess.run(
                ['nvidia-smi', '--query-gpu=index,name,memory.total,compute_cap,pci.bus_id',
                 '--format=csv,noheader,nounits'],
                capture_output=True,
                text=True,
                timeout=10
            )
            
            if result.returncode == 0:
                for line in result.stdout.strip().split('\n'):
                    if line:
                        parts = [p.strip() for p in line.split(',')]
                        if len(parts) >= 4:
                            gpu = GPUInfo(
                                vendor='nvidia',
                                name=parts[1],
                                index=int(parts[0]),
                                memory_mb=int(float(parts[2])),
                                compute_capability=parts[3],
                                pci_id=parts[4] if len(parts) > 4 else ""
                            )
                            gpus.append(gpu)
                            self.logger.info(f"Detected NVIDIA GPU: {gpu.name} ({gpu.compute_capability})")
        
        except FileNotFoundError:
            self.logger.debug("nvidia-smi not found, no NVIDIA GPUs detected")
        except Exception as e:
            self.logger.warning(f"Error detecting NVIDIA GPUs: {e}")
        
        return gpus
    
    def _detect_amd_gpus(self) -> List[GPUInfo]:
        """Detect AMD GPUs using rocm-smi or lspci"""
        gpus = []
        
        try:
            # Try rocm-smi first
            result = subprocess.run(
                ['rocm-smi', '--showproductname'],
                capture_output=True,
                text=True,
                timeout=10
            )
            
            if result.returncode == 0:
                # Parse rocm-smi output
                index = 0
                for line in result.stdout.split('\n'):
                    if 'Card series' in line or 'GPU' in line:
                        # Extract GPU name
                        match = re.search(r':\s*(.+)', line)
                        if match:
                            name = match.group(1).strip()
                            
                            # Try to get memory info
                            memory_mb = self._get_amd_memory_rocm(index)
                            
                            gpu = GPUInfo(
                                vendor='amd',
                                name=name,
                                index=index,
                                memory_mb=memory_mb
                            )
                            gpus.append(gpu)
                            index += 1
                            self.logger.info(f"Detected AMD GPU: {name}")
        
        except FileNotFoundError:
            # Fallback to lspci
            try:
                result = subprocess.run(
                    ['lspci', '-v'], capture_output=True, text=True, timeout=10
                )
                if result.returncode == 0:
                    index = 0
                    for line in result.stdout.split('\n'):
                        # Look for AMD VGA/3D/Display controllers
                        if 'AMD' in line and any(x in line for x in ['VGA', 'Display', '3D']):
                            # Extract GPU info from lspci
                            # Try to parse model name
                            name = 'AMD GPU'
                            
                            # Look for specific patterns
                            if 'Radeon RX' in line:
                                match = re.search(r'Radeon RX\s+\d+\w*\s*\w*', line)
                                if match:
                                    name = match.group(0)
                            elif 'Radeon VII' in line:
                                name = 'Radeon VII'
                            elif 'Vega' in line:
                                if 'Vega 64' in line:
                                    name = 'Vega 64'
                                elif 'Vega 56' in line:
                                    name = 'Vega 56'
                                else:
                                    name = 'Vega'
                            elif 'Instinct' in line:
                                match = re.search(r'Instinct\s+MI\d+\w*', line)
                                if match:
                                    name = match.group(0)
                            elif 'Radeon Pro' in line:
                                match = re.search(r'Radeon Pro\s+\w+\d+\w*', line)
                                if match:
                                    name = match.group(0)
                            elif '[' in line and ']' in line:
                                # Extract name from brackets
                                match = re.search(r'\[([^\]]+)\]', line)
                                if match:
                                    bracket_name = match.group(1)
                                    if 'Radeon' in bracket_name or 'RX' in bracket_name:
                                        name = bracket_name
                            
                            # Try to get memory info from lspci -v output
                            memory_mb = self._parse_amd_memory_lspci(result.stdout, line)
                            
                            gpu = GPUInfo(
                                vendor='amd',
                                name=name,
                                index=index,
                                memory_mb=memory_mb
                            )
                            gpus.append(gpu)
                            index += 1
                            self.logger.info(f"Detected AMD GPU: {name}")
            except Exception as e:
                self.logger.debug(f"lspci check failed: {e}")
        
        except Exception as e:
            self.logger.warning(f"Error detecting AMD GPUs: {e}")
        
        return gpus
    
    def _get_amd_memory_rocm(self, gpu_index: int) -> int:
        """Get AMD GPU memory using rocm-smi"""
        try:
            result = subprocess.run(
                ['rocm-smi', '--showmeminfo', 'vram', '--gpu', str(gpu_index)],
                capture_output=True,
                text=True,
                timeout=5
            )
            if result.returncode == 0:
                # Parse memory from output
                for line in result.stdout.split('\n'):
                    if 'Total' in line or 'VRAM' in line:
                        # Extract memory size (usually in MB or GB)
                        match = re.search(r'(\d+)\s*(MB|GB)', line, re.IGNORECASE)
                        if match:
                            size = int(match.group(1))
                            unit = match.group(2).upper()
                            if unit == 'GB':
                                return size * 1024
                            return size
        except Exception:
            pass
        return 0
    
    def _parse_amd_memory_lspci(self, lspci_output: str, device_line: str) -> int:
        """Parse AMD GPU memory from lspci verbose output"""
        try:
            # Find the memory line after the device
            lines = lspci_output.split('\n')
            device_idx = lines.index(device_line)
            
            # Look at the next few lines for memory info
            for i in range(device_idx + 1, min(device_idx + 10, len(lines))):
                line = lines[i]
                if 'Memory at' in line and 'prefetchable' in line:
                    # Extract size from memory range
                    match = re.search(r'\[size=(\d+)([KMG])\]', line)
                    if match:
                        size = int(match.group(1))
                        unit = match.group(2)
                        if unit == 'G':
                            return size * 1024
                        elif unit == 'M':
                            return size
                        elif unit == 'K':
                            return size // 1024
        except Exception:
            pass
        return 0
    
    def _detect_intel_gpus(self) -> List[GPUInfo]:
        """Detect Intel GPUs using xpu-smi or lspci"""
        gpus = []
        
        try:
            # Try xpu-smi for Intel Arc/Flex/Max
            result = subprocess.run(
                ['xpu-smi', 'discovery'],
                capture_output=True,
                text=True,
                timeout=10
            )
            
            if result.returncode == 0:
                index = 0
                for line in result.stdout.split('\n'):
                    if 'Device' in line and 'Intel' in line:
                        match = re.search(r'Device\s+(\d+).*?:\s*(.+)', line)
                        if match:
                            name = match.group(2).strip()
                            gpu = GPUInfo(
                                vendor='intel',
                                name=name,
                                index=index
                            )
                            gpus.append(gpu)
                            index += 1
                            self.logger.info(f"Detected Intel GPU: {name}")
        
        except FileNotFoundError:
            # Fallback to lspci
            try:
                result = subprocess.run(
                    ['lspci'], capture_output=True, text=True, timeout=10
                )
                if result.returncode == 0:
                    index = 0
                    for line in result.stdout.split('\n'):
                        if 'Intel' in line and ('VGA' in line or 'Display' in line or '3D' in line):
                            # Look for Arc, Flex, Max series
                            if any(x in line for x in ['Arc', 'Flex', 'Max', 'Xe']):
                                match = re.search(r'Intel[^:]*:\s*(.+)', line)
                                if match:
                                    name = match.group(1).strip()
                                    gpu = GPUInfo(
                                        vendor='intel',
                                        name=name,
                                        index=index
                                    )
                                    gpus.append(gpu)
                                    index += 1
                                    self.logger.info(f"Detected Intel GPU: {name}")
            except Exception as e:
                self.logger.debug(f"lspci check failed: {e}")
        
        except Exception as e:
            self.logger.warning(f"Error detecting Intel GPUs: {e}")
        
        return gpus
    
    def _is_apple_silicon(self) -> bool:
        """Check if running on Apple Silicon"""
        try:
            import platform
            return platform.system() == 'Darwin' and platform.machine() == 'arm64'
        except Exception:
            return False
    
    def _detect_apple_gpu(self) -> Optional[GPUInfo]:
        """Detect Apple Silicon GPU"""
        try:
            result = subprocess.run(
                ['sysctl', '-n', 'machdep.cpu.brand_string'],
                capture_output=True,
                text=True,
                timeout=5
            )
            
            if result.returncode == 0:
                cpu_name = result.stdout.strip()
                # Check for M1, M2, M3
                for chip in self.APPLE_CHIPS:
                    if chip in cpu_name:
                        gpu = GPUInfo(
                            vendor='apple',
                            name=f"Apple {chip} GPU",
                            index=0
                        )
                        self.logger.info(f"Detected Apple Silicon: {gpu.name}")
                        return gpu
        
        except Exception as e:
            self.logger.debug(f"Apple Silicon detection failed: {e}")
        
        return None
    
    def _classify_gpu_capabilities(self, gpu: GPUInfo):
        """
        Classify GPU for inference and training capabilities
        
        Args:
            gpu: GPUInfo to classify
        """
        # NVIDIA classification
        if gpu.vendor == 'nvidia':
            # Check compute capability for tensor cores
            try:
                compute_cap = float(gpu.compute_capability)
                if compute_cap >= self.NVIDIA_MIN_COMPUTE:
                    gpu.is_training_capable = True
                    gpu.is_inference_capable = True
                else:
                    # Older GPUs can still do inference
                    gpu.is_inference_capable = compute_cap >= 6.0
            except ValueError:
                pass
            
            # Check if it's an inference-optimized GPU
            for inference_model in self.INFERENCE_GPUS['nvidia']:
                if inference_model in gpu.name:
                    gpu.is_inference_capable = True
                    break
        
        # AMD classification
        elif gpu.vendor == 'amd':
            # ROCm 5.0+ supports many consumer and professional cards
            # Check for inference-optimized models
            is_inference_optimized = False
            is_training_ready = False
            
            for inference_model in self.INFERENCE_GPUS['amd']:
                if inference_model in gpu.name:
                    gpu.is_inference_capable = True
                    is_inference_optimized = True
                    # MI series and Pro series are training ready
                    if 'MI' in gpu.name or 'Pro' in gpu.name or 'Instinct' in gpu.name:
                        gpu.is_training_capable = True
                        is_training_ready = True
                    break
            
            # Check ROCm-capable consumer cards
            if not is_inference_optimized:
                for rocm_model in self.AMD_ROCM_CAPABLE:
                    if rocm_model in gpu.name:
                        gpu.is_inference_capable = True
                        # RDNA2 (6000 series) and RDNA3 (7000 series) support training
                        if any(x in gpu.name for x in ['RX 6', 'RX 7', 'Vega', 'VII']):
                            gpu.is_training_capable = True
                        break
            
            # Additional checks for specific series
            if 'RDNA' in gpu.name.upper() or 'RDNA2' in gpu.name.upper() or 'RDNA3' in gpu.name.upper():
                gpu.is_inference_capable = True
                gpu.is_training_capable = True
            
            # Vega architecture and newer support inference
            if 'Vega' in gpu.name or 'VII' in gpu.name:
                gpu.is_inference_capable = True
                gpu.is_training_capable = True
            
            # Check for minimum memory (4GB for inference, 8GB for training)
            if gpu.memory_mb > 0:
                if gpu.memory_mb < 4096:
                    gpu.is_inference_capable = False
                    gpu.is_training_capable = False
                elif gpu.memory_mb < 8192:
                    gpu.is_training_capable = False
        
        # Intel classification
        elif gpu.vendor == 'intel':
            # Arc, Flex, Max all support XMX (Xe Matrix Extensions)
            for series in self.INFERENCE_GPUS['intel']:
                if series in gpu.name:
                    gpu.is_inference_capable = True
                    gpu.is_training_capable = True
                    break
        
        # Apple Silicon
        elif gpu.vendor == 'apple':
            # All Apple Silicon supports ML acceleration
            gpu.is_inference_capable = True
            gpu.is_training_capable = True
    
    def get_inference_gpus(self) -> List[GPUInfo]:
        """Get list of GPUs suitable for inference"""
        return [gpu for gpu in self.gpus if gpu.is_inference_capable]
    
    def get_training_gpus(self) -> List[GPUInfo]:
        """Get list of GPUs suitable for training"""
        return [gpu for gpu in self.gpus if gpu.is_training_capable]
    
    def get_gpu_labels(self, gpu: GPUInfo) -> List[str]:
        """
        Get appropriate runner labels for a GPU
        
        Args:
            gpu: GPU to get labels for
            
        Returns:
            List of labels
        """
        labels = ['self-hosted', 'gpu']
        
        # Add vendor label
        labels.append(gpu.vendor)
        
        # Add capability labels
        if gpu.is_inference_capable:
            labels.append('inference')
        if gpu.is_training_capable:
            labels.append('training')
        
        # Add specific model info
        if gpu.vendor == 'nvidia':
            if 'T4' in gpu.name:
                labels.append('t4')
            elif 'A100' in gpu.name or 'A10' in gpu.name:
                labels.append('ampere')
            elif 'V100' in gpu.name:
                labels.append('volta')
        
        elif gpu.vendor == 'amd':
            if 'MI' in gpu.name:
                labels.append('mi-series')
                # Add specific MI model
                if 'MI300' in gpu.name:
                    labels.append('mi300')
                elif 'MI250' in gpu.name:
                    labels.append('mi250')
                elif 'MI210' in gpu.name:
                    labels.append('mi210')
            elif 'RX 7' in gpu.name:
                labels.append('rdna3')
                labels.append('rx7000')
            elif 'RX 6' in gpu.name:
                labels.append('rdna2')
                labels.append('rx6000')
            elif 'Vega' in gpu.name or 'VII' in gpu.name:
                labels.append('vega')
            elif 'Radeon Pro' in gpu.name:
                labels.append('radeon-pro')
        
        elif gpu.vendor == 'intel':
            if 'Arc' in gpu.name:
                labels.append('arc')
            elif 'Flex' in gpu.name:
                labels.append('flex')
            elif 'Max' in gpu.name:
                labels.append('max')
        
        elif gpu.vendor == 'apple':
            for chip in self.APPLE_CHIPS:
                if chip in gpu.name:
                    labels.append(chip.lower())
                    break
        
        return labels
