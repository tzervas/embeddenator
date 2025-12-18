"""
Resource Optimization Module

Automatically calculates optimal resource allocation for runners
while preserving sufficient resources for the host system.
"""

import logging
import os
import subprocess
from typing import Dict, List, Optional, Tuple

# psutil is optional - use fallback if not available
try:
    import psutil
    HAS_PSUTIL = True
except ImportError:
    HAS_PSUTIL = False
    # Fallback implementations
    class _FakePsutil:
        @staticmethod
        def cpu_count(logical=True):
            try:
                if logical:
                    return os.cpu_count()
                else:
                    # Try to get physical cores from /proc/cpuinfo
                    with open('/proc/cpuinfo') as f:
                        return len(set([line.split(':')[1].strip() for line in f if 'physical id' in line]))
            except:
                return os.cpu_count() or 4
        
        @staticmethod
        def virtual_memory():
            class Mem:
                def __init__(self):
                    try:
                        with open('/proc/meminfo') as f:
                            for line in f:
                                if 'MemTotal' in line:
                                    self.total = int(line.split()[1]) * 1024  # Convert KB to bytes
                                    break
                    except:
                        self.total = 8 * 1024**3  # Default 8GB
            return Mem()
        
        @staticmethod
        def disk_usage(path):
            class Disk:
                def __init__(self, path):
                    try:
                        import shutil
                        usage = shutil.disk_usage(path)
                        self.total = usage.total
                    except:
                        self.total = 100 * 1024**3  # Default 100GB
            return Disk(path)
        
        @staticmethod
        def cpu_freq():
            return None
    
    psutil = _FakePsutil()


class CPUInfo:
    """Information about CPU capabilities"""
    
    def __init__(self):
        self.model = ""
        self.cores_physical = 0
        self.cores_logical = 0
        self.threads_per_core = 0
        self.frequency_mhz = 0
        self.is_xeon = False
        self.is_epyc = False
        self.is_apple_silicon = False
        self.supports_avx2 = False
        self.supports_avx512 = False


class ResourceOptimizer:
    """Optimize resource allocation for runners"""
    
    # Resource reservation for host (minimum to keep system functional)
    MIN_HOST_CPU_CORES = 2
    MIN_HOST_MEMORY_GB = 2
    MIN_HOST_DISK_GB = 10
    
    # Recommended reservations (percentage of total)
    RECOMMENDED_HOST_CPU_PERCENT = 20  # Reserve 20% for host
    RECOMMENDED_HOST_MEMORY_PERCENT = 15  # Reserve 15% for host
    
    # Intel Xeon E5-2660 v3/v4 specific optimizations
    XEON_E5_2660_CORES = 10  # per CPU
    XEON_E5_2660_THREADS = 20  # with HT
    
    def __init__(self, logger: logging.Logger):
        """
        Initialize resource optimizer
        
        Args:
            logger: Logger instance
        """
        self.logger = logger
        self.cpu_info = self._detect_cpu_info()
        self.total_memory_gb = psutil.virtual_memory().total / (1024**3)
        self.total_disk_gb = psutil.disk_usage('/').total / (1024**3)
    
    def _detect_cpu_info(self) -> CPUInfo:
        """Detect CPU capabilities"""
        info = CPUInfo()
        
        try:
            # Get CPU info
            info.cores_physical = psutil.cpu_count(logical=False)
            info.cores_logical = psutil.cpu_count(logical=True)
            info.threads_per_core = info.cores_logical // info.cores_physical if info.cores_physical else 1
            
            # Get CPU frequency
            cpu_freq = psutil.cpu_freq()
            if cpu_freq:
                info.frequency_mhz = cpu_freq.max if cpu_freq.max else cpu_freq.current
            
            # Detect CPU model and features
            if os.path.exists('/proc/cpuinfo'):
                with open('/proc/cpuinfo', 'r') as f:
                    cpuinfo = f.read()
                    
                    # Extract model name
                    for line in cpuinfo.split('\n'):
                        if 'model name' in line.lower():
                            info.model = line.split(':')[1].strip()
                            break
                    
                    # Check for specific CPUs
                    info.is_xeon = 'Xeon' in info.model
                    info.is_epyc = 'EPYC' in info.model
                    
                    # Check CPU flags
                    if 'flags' in cpuinfo:
                        flags_line = [l for l in cpuinfo.split('\n') if l.startswith('flags')][0]
                        flags = flags_line.split(':')[1].strip().split()
                        info.supports_avx2 = 'avx2' in flags
                        info.supports_avx512 = any('avx512' in f for f in flags)
            
            # Check for Apple Silicon
            try:
                import platform
                if platform.system() == 'Darwin' and platform.machine() == 'arm64':
                    info.is_apple_silicon = True
                    result = subprocess.run(
                        ['sysctl', '-n', 'machdep.cpu.brand_string'],
                        capture_output=True, text=True, timeout=5
                    )
                    if result.returncode == 0:
                        info.model = result.stdout.strip()
            except Exception:
                pass
            
            self.logger.info(f"Detected CPU: {info.model}")
            self.logger.info(f"  Physical cores: {info.cores_physical}, Logical: {info.cores_logical}")
            
        except Exception as e:
            self.logger.error(f"Error detecting CPU info: {e}")
        
        return info
    
    def is_xeon_e5_2660(self) -> bool:
        """Check if running on Intel Xeon E5-2660 v3 or v4"""
        model = self.cpu_info.model.lower()
        return 'e5-2660' in model and ('v3' in model or 'v4' in model)
    
    def calculate_optimal_resources(self, runner_count: int, 
                                   enable_optimization: bool = True,
                                   gpu_count: int = 0) -> Dict:
        """
        Calculate optimal resource allocation per runner
        
        Args:
            runner_count: Number of runners to deploy
            enable_optimization: Enable automatic optimization
            gpu_count: Number of GPUs to distribute
            
        Returns:
            Dictionary with resource allocation
        """
        if not enable_optimization:
            return {
                'cpu_cores_per_runner': 0,  # Unlimited
                'memory_gb_per_runner': 0,  # Unlimited
                'host_cpu_cores': 0,
                'host_memory_gb': 0,
                'optimization_enabled': False
            }
        
        # Calculate host reservations
        host_cpu = max(
            self.MIN_HOST_CPU_CORES,
            int(self.cpu_info.cores_physical * self.RECOMMENDED_HOST_CPU_PERCENT / 100)
        )
        
        host_memory = max(
            self.MIN_HOST_MEMORY_GB,
            self.total_memory_gb * self.RECOMMENDED_HOST_MEMORY_PERCENT / 100
        )
        
        # Calculate available resources for runners
        available_cpu = self.cpu_info.cores_physical - host_cpu
        available_memory = self.total_memory_gb - host_memory
        
        # Ensure we don't over-allocate
        available_cpu = max(1, available_cpu)
        available_memory = max(1, available_memory)
        
        # Calculate per-runner allocation
        cpu_per_runner = max(1, available_cpu // runner_count)
        memory_per_runner = max(1, available_memory / runner_count)
        
        # Special optimization for Xeon E5-2660 v3/v4
        if self.is_xeon_e5_2660():
            self.logger.info("Detected Intel Xeon E5-2660 v3/v4 - applying optimizations")
            # With dual socket, we have 20 physical cores (40 threads)
            # Optimal is to use physical cores, not hyperthreads for compute
            cpu_per_runner = min(cpu_per_runner, 4)  # 4 physical cores per runner
            self.logger.info(f"  Xeon optimization: {cpu_per_runner} cores per runner")
        
        # Apple Silicon optimization
        if self.cpu_info.is_apple_silicon:
            self.logger.info("Detected Apple Silicon - applying performance core optimization")
            # M1/M2/M3 have performance and efficiency cores
            # Reserve efficiency cores for host, use performance for runners
            performance_cores = self.cpu_info.cores_physical // 2  # Rough estimate
            cpu_per_runner = max(2, performance_cores // runner_count)
        
        # GPU distribution
        gpu_per_runner = 0
        if gpu_count > 0:
            # Distribute GPUs across runners
            gpu_per_runner = gpu_count / runner_count
            self.logger.info(f"GPU distribution: {gpu_per_runner:.2f} GPUs per runner")
        
        allocation = {
            'cpu_cores_per_runner': cpu_per_runner,
            'memory_gb_per_runner': round(memory_per_runner, 1),
            'host_cpu_cores': host_cpu,
            'host_memory_gb': round(host_memory, 1),
            'total_cpu_cores': self.cpu_info.cores_physical,
            'total_memory_gb': round(self.total_memory_gb, 1),
            'available_cpu_cores': available_cpu,
            'available_memory_gb': round(available_memory, 1),
            'gpu_per_runner': gpu_per_runner,
            'optimization_enabled': True,
            'cpu_model': self.cpu_info.model,
            'is_xeon_e5_2660': self.is_xeon_e5_2660(),
            'is_apple_silicon': self.cpu_info.is_apple_silicon
        }
        
        self.logger.info("Resource allocation calculated:")
        self.logger.info(f"  Total CPU cores: {self.cpu_info.cores_physical}")
        self.logger.info(f"  Host reserved: {host_cpu} cores, {host_memory:.1f} GB")
        self.logger.info(f"  Per runner: {cpu_per_runner} cores, {memory_per_runner:.1f} GB")
        if gpu_count > 0:
            self.logger.info(f"  GPU allocation: {gpu_per_runner:.2f} per runner")
        
        return allocation
    
    def get_cpu_affinity(self, runner_id: int, cores_per_runner: int) -> List[int]:
        """
        Calculate CPU affinity for a runner
        
        Args:
            runner_id: Runner ID (1-indexed)
            cores_per_runner: Number of cores allocated per runner
            
        Returns:
            List of CPU core IDs to bind to
        """
        # Reserve first cores for host
        host_cores = max(
            self.MIN_HOST_CPU_CORES,
            int(self.cpu_info.cores_physical * self.RECOMMENDED_HOST_CPU_PERCENT / 100)
        )
        
        # Calculate starting core for this runner
        start_core = host_cores + ((runner_id - 1) * cores_per_runner)
        end_core = start_core + cores_per_runner
        
        # Ensure we don't exceed available cores
        end_core = min(end_core, self.cpu_info.cores_physical)
        
        cpu_list = list(range(start_core, end_core))
        
        # For Xeon with HT, also include corresponding hyperthreads
        if self.cpu_info.threads_per_core == 2:
            hyperthread_offset = self.cpu_info.cores_physical
            hyperthread_list = [c + hyperthread_offset for c in cpu_list]
            cpu_list.extend(hyperthread_list)
        
        return cpu_list
    
    def generate_systemd_config(self, runner_name: str, allocation: Dict, 
                                runner_id: int) -> str:
        """
        Generate systemd service configuration with resource limits
        
        Args:
            runner_name: Name of the runner
            allocation: Resource allocation from calculate_optimal_resources
            runner_id: Runner ID
            
        Returns:
            Systemd service unit configuration as string
        """
        cpu_cores = allocation['cpu_cores_per_runner']
        memory_gb = allocation['memory_gb_per_runner']
        
        # Get CPU affinity
        cpu_affinity = self.get_cpu_affinity(runner_id, cpu_cores)
        cpu_affinity_str = ','.join(map(str, cpu_affinity))
        
        config = f"""[Unit]
Description=GitHub Actions Runner - {runner_name}
After=network.target

[Service]
Type=simple
User=runner
WorkingDirectory=/home/runner/actions-runner-{runner_name}
ExecStart=/home/runner/actions-runner-{runner_name}/run.sh
Restart=always
RestartSec=10

# Resource Limits
CPUQuota={cpu_cores * 100}%
MemoryMax={int(memory_gb * 1024)}M
MemoryHigh={int(memory_gb * 1024 * 0.9)}M

# CPU Affinity
CPUAffinity={cpu_affinity_str}

# Priority (nice value)
Nice=5

[Install]
WantedBy=multi-user.target
"""
        return config
    
    def get_docker_resource_flags(self, allocation: Dict, runner_id: int) -> List[str]:
        """
        Generate Docker resource flags for containerized runners
        
        Args:
            allocation: Resource allocation from calculate_optimal_resources
            runner_id: Runner ID
            
        Returns:
            List of Docker CLI flags
        """
        flags = []
        
        cpu_cores = allocation['cpu_cores_per_runner']
        memory_gb = allocation['memory_gb_per_runner']
        
        if cpu_cores > 0:
            # Set CPU quota (in microseconds per 100ms period)
            cpu_quota = int(cpu_cores * 100000)
            flags.extend(['--cpu-period', '100000', '--cpu-quota', str(cpu_quota)])
            
            # Set CPU affinity
            cpu_affinity = self.get_cpu_affinity(runner_id, cpu_cores)
            cpu_set = ','.join(map(str, cpu_affinity))
            flags.extend(['--cpuset-cpus', cpu_set])
        
        if memory_gb > 0:
            # Set memory limit
            memory_bytes = int(memory_gb * 1024 * 1024 * 1024)
            flags.extend(['--memory', str(memory_bytes)])
            
            # Set memory reservation (soft limit)
            memory_reservation = int(memory_bytes * 0.8)
            flags.extend(['--memory-reservation', str(memory_reservation)])
        
        return flags
    
    def validate_allocation(self, allocation: Dict, runner_count: int) -> Tuple[bool, List[str]]:
        """
        Validate that resource allocation is feasible
        
        Args:
            allocation: Resource allocation to validate
            runner_count: Number of runners
            
        Returns:
            Tuple of (is_valid, list of warning messages)
        """
        warnings = []
        
        # Check if total allocation exceeds available resources
        total_cpu_needed = allocation['cpu_cores_per_runner'] * runner_count + allocation['host_cpu_cores']
        if total_cpu_needed > self.cpu_info.cores_physical:
            warnings.append(
                f"CPU over-allocation: Need {total_cpu_needed} cores but only "
                f"{self.cpu_info.cores_physical} available"
            )
        
        total_memory_needed = allocation['memory_gb_per_runner'] * runner_count + allocation['host_memory_gb']
        if total_memory_needed > self.total_memory_gb:
            warnings.append(
                f"Memory over-allocation: Need {total_memory_needed:.1f} GB but only "
                f"{self.total_memory_gb:.1f} GB available"
            )
        
        # Check if host reservation is too low
        if allocation['host_cpu_cores'] < self.MIN_HOST_CPU_CORES:
            warnings.append(
                f"Host CPU reservation ({allocation['host_cpu_cores']}) below minimum "
                f"({self.MIN_HOST_CPU_CORES})"
            )
        
        if allocation['host_memory_gb'] < self.MIN_HOST_MEMORY_GB:
            warnings.append(
                f"Host memory reservation ({allocation['host_memory_gb']:.1f} GB) below minimum "
                f"({self.MIN_HOST_MEMORY_GB} GB)"
            )
        
        is_valid = len(warnings) == 0
        return is_valid, warnings
