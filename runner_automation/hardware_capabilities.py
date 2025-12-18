"""
Hardware Capabilities Configuration

Defines supported CPUs, GPUs, and their capabilities for runner optimization.
This file can be easily updated as new hardware becomes available.
"""

# ============================================================================
# CPU Configurations
# ============================================================================

CPU_CONFIGURATIONS = {
    # Intel CPUs - 10th Gen and newer
    'intel': {
        # 10th Gen (Comet Lake) - 2020
        '10th_gen': {
            'models': ['i9-10900K', 'i9-10900', 'i7-10700K', 'i7-10700', 'i5-10600K', 'i5-10400'],
            'microarchitecture': 'Comet Lake',
            'max_cores': 10,
            'hyperthreading': True,
            'avx512': False,
            'release_year': 2020,
            'inference_capable': True,
            'training_capable': False,
        },
        # 11th Gen (Rocket Lake) - 2021
        '11th_gen': {
            'models': ['i9-11900K', 'i9-11900', 'i7-11700K', 'i7-11700', 'i5-11600K', 'i5-11400'],
            'microarchitecture': 'Rocket Lake',
            'max_cores': 8,
            'hyperthreading': True,
            'avx512': True,
            'release_year': 2021,
            'inference_capable': True,
            'training_capable': True,
        },
        # 12th Gen (Alder Lake) - 2021
        '12th_gen': {
            'models': ['i9-12900K', 'i9-12900', 'i7-12700K', 'i7-12700', 'i5-12600K', 'i5-12400'],
            'microarchitecture': 'Alder Lake',
            'max_cores': 16,  # P+E cores
            'hyperthreading': True,
            'avx512': False,  # Removed in Alder Lake
            'p_cores': True,  # Performance cores
            'e_cores': True,  # Efficiency cores
            'release_year': 2021,
            'inference_capable': True,
            'training_capable': True,
        },
        # 13th Gen (Raptor Lake) - 2022
        '13th_gen': {
            'models': ['i9-13900K', 'i9-13900', 'i7-13700K', 'i7-13700', 'i5-13600K', 'i5-13400'],
            'microarchitecture': 'Raptor Lake',
            'max_cores': 24,  # P+E cores
            'hyperthreading': True,
            'avx512': False,
            'p_cores': True,
            'e_cores': True,
            'release_year': 2022,
            'inference_capable': True,
            'training_capable': True,
        },
        # 14th Gen (Raptor Lake Refresh) - 2023
        '14th_gen': {
            'models': ['i9-14900K', 'i9-14900', 'i7-14700K', 'i7-14700', 'i5-14600K', 'i5-14400'],
            'microarchitecture': 'Raptor Lake Refresh',
            'max_cores': 24,
            'hyperthreading': True,
            'avx512': False,
            'p_cores': True,
            'e_cores': True,
            'release_year': 2023,
            'inference_capable': True,
            'training_capable': True,
        },
        # Xeon Scalable (Ice Lake and newer)
        'xeon_ice_lake': {
            'models': ['Xeon Gold 6338', 'Xeon Platinum 8380', 'Xeon Silver 4314'],
            'microarchitecture': 'Ice Lake',
            'max_cores': 40,
            'hyperthreading': True,
            'avx512': True,
            'release_year': 2021,
            'inference_capable': True,
            'training_capable': True,
        },
        'xeon_sapphire_rapids': {
            'models': ['Xeon Platinum 8480', 'Xeon Gold 6448Y'],
            'microarchitecture': 'Sapphire Rapids',
            'max_cores': 60,
            'hyperthreading': True,
            'avx512': True,
            'amx': True,  # Advanced Matrix Extensions
            'release_year': 2023,
            'inference_capable': True,
            'training_capable': True,
        },
    },
    
    # AMD CPUs - Zen 1 and newer
    'amd': {
        # Zen 1 (Ryzen 1000, EPYC 7001) - 2017
        'zen1': {
            'models': ['Ryzen 7 1800X', 'Ryzen 7 1700X', 'Ryzen 5 1600X', 'EPYC 7601', 'EPYC 7551'],
            'microarchitecture': 'Zen',
            'max_cores': 32,
            'hyperthreading': True,
            'avx2': True,
            'release_year': 2017,
            'inference_capable': True,
            'training_capable': False,
        },
        # Zen+ (Ryzen 2000) - 2018
        'zen_plus': {
            'models': ['Ryzen 7 2700X', 'Ryzen 5 2600X'],
            'microarchitecture': 'Zen+',
            'max_cores': 8,
            'hyperthreading': True,
            'avx2': True,
            'release_year': 2018,
            'inference_capable': True,
            'training_capable': False,
        },
        # Zen 2 (Ryzen 3000, EPYC 7002) - 2019
        'zen2': {
            'models': ['Ryzen 9 3950X', 'Ryzen 9 3900X', 'Ryzen 7 3800X', 'EPYC 7742', 'EPYC 7702'],
            'microarchitecture': 'Zen 2',
            'max_cores': 64,
            'hyperthreading': True,
            'avx2': True,
            'release_year': 2019,
            'inference_capable': True,
            'training_capable': True,
        },
        # Zen 3 (Ryzen 5000, EPYC 7003) - 2020
        'zen3': {
            'models': ['Ryzen 9 5950X', 'Ryzen 9 5900X', 'Ryzen 7 5800X', 'EPYC 7763', 'EPYC 7713'],
            'microarchitecture': 'Zen 3',
            'max_cores': 64,
            'hyperthreading': True,
            'avx2': True,
            'release_year': 2020,
            'inference_capable': True,
            'training_capable': True,
        },
        # Zen 3+ (Ryzen 6000) - 2022
        'zen3_plus': {
            'models': ['Ryzen 9 6900HX', 'Ryzen 7 6800H'],
            'microarchitecture': 'Zen 3+',
            'max_cores': 8,
            'hyperthreading': True,
            'avx2': True,
            'release_year': 2022,
            'inference_capable': True,
            'training_capable': True,
        },
        # Zen 4 (Ryzen 7000, EPYC 9004) - 2022
        'zen4': {
            'models': ['Ryzen 9 7950X', 'Ryzen 9 7900X', 'Ryzen 7 7700X', 'EPYC 9654', 'EPYC 9554'],
            'microarchitecture': 'Zen 4',
            'max_cores': 96,
            'hyperthreading': True,
            'avx512': True,  # First Zen with AVX-512
            'release_year': 2022,
            'inference_capable': True,
            'training_capable': True,
        },
        # Zen 5 (Ryzen 9000) - 2024
        'zen5': {
            'models': ['Ryzen 9 9950X', 'Ryzen 9 9900X', 'Ryzen 7 9700X'],
            'microarchitecture': 'Zen 5',
            'max_cores': 16,
            'hyperthreading': True,
            'avx512': True,
            'release_year': 2024,
            'inference_capable': True,
            'training_capable': True,
        },
    },
}

# CPU feature requirements for workloads
CPU_WORKLOAD_REQUIREMENTS = {
    'inference': {
        'min_cores': 2,
        'preferred_features': ['avx2', 'avx512'],
        'min_year': 2017,
    },
    'training': {
        'min_cores': 4,
        'preferred_features': ['avx2', 'avx512', 'amx'],
        'min_year': 2019,
    },
}

# ============================================================================
# GPU Configurations
# ============================================================================

GPU_CONFIGURATIONS = {
    # NVIDIA GPUs
    'nvidia': {
        # Data Center / Professional
        'data_center': {
            'models': [
                # Hopper (2022+)
                {'name': 'H100', 'memory_gb': 80, 'compute': 9.0, 'tensor_cores': True, 'inference': True, 'training': True},
                {'name': 'H800', 'memory_gb': 80, 'compute': 9.0, 'tensor_cores': True, 'inference': True, 'training': True},
                # Ampere (2020+)
                {'name': 'A100', 'memory_gb': 80, 'compute': 8.0, 'tensor_cores': True, 'inference': True, 'training': True},
                {'name': 'A40', 'memory_gb': 48, 'compute': 8.6, 'tensor_cores': True, 'inference': True, 'training': True},
                {'name': 'A30', 'memory_gb': 24, 'compute': 8.0, 'tensor_cores': True, 'inference': True, 'training': True},
                {'name': 'A10', 'memory_gb': 24, 'compute': 8.6, 'tensor_cores': True, 'inference': True, 'training': False},
                {'name': 'A16', 'memory_gb': 16, 'compute': 8.6, 'tensor_cores': True, 'inference': True, 'training': False},
                {'name': 'A2', 'memory_gb': 16, 'compute': 8.6, 'tensor_cores': True, 'inference': True, 'training': False},
                # Inference Optimized
                {'name': 'T4', 'memory_gb': 16, 'compute': 7.5, 'tensor_cores': True, 'inference': True, 'training': False},
                {'name': 'L4', 'memory_gb': 24, 'compute': 8.9, 'tensor_cores': True, 'inference': True, 'training': False},
                {'name': 'L40', 'memory_gb': 48, 'compute': 8.9, 'tensor_cores': True, 'inference': True, 'training': True},
                {'name': 'L40S', 'memory_gb': 48, 'compute': 8.9, 'tensor_cores': True, 'inference': True, 'training': True},
            ],
        },
        # Professional / Workstation
        'professional': {
            'models': [
                # RTX Ada Generation
                {'name': 'RTX 6000 Ada', 'memory_gb': 48, 'compute': 8.9, 'tensor_cores': True, 'inference': True, 'training': True},
                {'name': 'RTX 5880 Ada', 'memory_gb': 48, 'compute': 8.9, 'tensor_cores': True, 'inference': True, 'training': True},
                {'name': 'RTX 5000 Ada', 'memory_gb': 32, 'compute': 8.9, 'tensor_cores': True, 'inference': True, 'training': True},
                {'name': 'RTX 4500 Ada', 'memory_gb': 24, 'compute': 8.9, 'tensor_cores': True, 'inference': True, 'training': True},
                {'name': 'RTX 4000 Ada', 'memory_gb': 20, 'compute': 8.9, 'tensor_cores': True, 'inference': True, 'training': False},
                # RTX Ampere Generation
                {'name': 'RTX A6000', 'memory_gb': 48, 'compute': 8.6, 'tensor_cores': True, 'inference': True, 'training': True},
                {'name': 'RTX A5500', 'memory_gb': 24, 'compute': 8.6, 'tensor_cores': True, 'inference': True, 'training': True},
                {'name': 'RTX A5000', 'memory_gb': 24, 'compute': 8.6, 'tensor_cores': True, 'inference': True, 'training': True},
                {'name': 'RTX A4500', 'memory_gb': 20, 'compute': 8.6, 'tensor_cores': True, 'inference': True, 'training': True},
                {'name': 'RTX A4000', 'memory_gb': 16, 'compute': 8.6, 'tensor_cores': True, 'inference': True, 'training': False},
                {'name': 'RTX A2000', 'memory_gb': 12, 'compute': 8.6, 'tensor_cores': True, 'inference': True, 'training': False},
                # Quadro (Legacy Professional)
                {'name': 'Quadro RTX 8000', 'memory_gb': 48, 'compute': 7.5, 'tensor_cores': True, 'inference': True, 'training': True},
                {'name': 'Quadro RTX 6000', 'memory_gb': 24, 'compute': 7.5, 'tensor_cores': True, 'inference': True, 'training': True},
                {'name': 'Quadro RTX 5000', 'memory_gb': 16, 'compute': 7.5, 'tensor_cores': True, 'inference': True, 'training': False},
            ],
        },
        # Consumer / Gaming
        'consumer': {
            'models': [
                # RTX 40 Series (Ada Lovelace)
                {'name': 'RTX 4090', 'memory_gb': 24, 'compute': 8.9, 'tensor_cores': True, 'inference': True, 'training': True},
                {'name': 'RTX 4080', 'memory_gb': 16, 'compute': 8.9, 'tensor_cores': True, 'inference': True, 'training': True},
                {'name': 'RTX 4070', 'memory_gb': 12, 'compute': 8.9, 'tensor_cores': True, 'inference': True, 'training': False},
                # RTX 30 Series (Ampere)
                {'name': 'RTX 3090', 'memory_gb': 24, 'compute': 8.6, 'tensor_cores': True, 'inference': True, 'training': True},
                {'name': 'RTX 3080', 'memory_gb': 12, 'compute': 8.6, 'tensor_cores': True, 'inference': True, 'training': True},
                {'name': 'RTX 3070', 'memory_gb': 8, 'compute': 8.6, 'tensor_cores': True, 'inference': True, 'training': False},
                {'name': 'RTX 3060', 'memory_gb': 12, 'compute': 8.6, 'tensor_cores': True, 'inference': True, 'training': False},
            ],
        },
    },
    
    # AMD GPUs
    'amd': {
        # Data Center / Professional
        'data_center': {
            'models': [
                # CDNA 3 (2023+)
                {'name': 'MI300X', 'memory_gb': 192, 'inference': True, 'training': True, 'rocm': '6.0'},
                {'name': 'MI300A', 'memory_gb': 128, 'inference': True, 'training': True, 'rocm': '6.0'},
                # CDNA 2 (2021+)
                {'name': 'MI250X', 'memory_gb': 128, 'inference': True, 'training': True, 'rocm': '5.0'},
                {'name': 'MI250', 'memory_gb': 128, 'inference': True, 'training': True, 'rocm': '5.0'},
                {'name': 'MI210', 'memory_gb': 64, 'inference': True, 'training': True, 'rocm': '5.0'},
                # CDNA 1 (2020)
                {'name': 'MI100', 'memory_gb': 32, 'inference': True, 'training': True, 'rocm': '4.0'},
                # Legacy Instinct
                {'name': 'MI60', 'memory_gb': 32, 'inference': True, 'training': True, 'rocm': '3.0'},
                {'name': 'MI50', 'memory_gb': 16, 'inference': True, 'training': True, 'rocm': '2.0'},
            ],
        },
        # Professional / Workstation
        'professional': {
            'models': [
                # Radeon Pro W7000 Series (RDNA 3)
                {'name': 'Radeon Pro W7900', 'memory_gb': 48, 'inference': True, 'training': True, 'rocm': '5.5'},
                {'name': 'Radeon Pro W7800', 'memory_gb': 32, 'inference': True, 'training': True, 'rocm': '5.5'},
                {'name': 'Radeon Pro W7700', 'memory_gb': 16, 'inference': True, 'training': False, 'rocm': '5.5'},
                {'name': 'Radeon Pro W7600', 'memory_gb': 8, 'inference': True, 'training': False, 'rocm': '5.5'},
                # Radeon Pro W6000 Series (RDNA 2)
                {'name': 'Radeon Pro W6900X', 'memory_gb': 32, 'inference': True, 'training': True, 'rocm': '5.0'},
                {'name': 'Radeon Pro W6800', 'memory_gb': 32, 'inference': True, 'training': True, 'rocm': '5.0'},
                {'name': 'Radeon Pro W6600', 'memory_gb': 8, 'inference': True, 'training': False, 'rocm': '5.0'},
                # Radeon Pro V-Series (Data Center Visualization)
                {'name': 'Radeon Pro V620', 'memory_gb': 32, 'inference': True, 'training': True, 'rocm': '5.0'},
                {'name': 'Radeon Pro V520', 'memory_gb': 8, 'inference': True, 'training': False, 'rocm': '5.0'},
                # Legacy Radeon Pro WX Series
                {'name': 'Radeon Pro WX 9100', 'memory_gb': 16, 'inference': True, 'training': False, 'rocm': '3.0'},
                {'name': 'Radeon Pro WX 8200', 'memory_gb': 8, 'inference': True, 'training': False, 'rocm': '3.0'},
            ],
        },
        # Consumer / Gaming
        'consumer': {
            'models': [
                # RDNA 3 (RX 7000 series)
                {'name': 'RX 7900 XTX', 'memory_gb': 24, 'inference': True, 'training': False, 'rocm': '5.5'},
                {'name': 'RX 7900 XT', 'memory_gb': 20, 'inference': True, 'training': False, 'rocm': '5.5'},
                {'name': 'RX 7900 GRE', 'memory_gb': 16, 'inference': True, 'training': False, 'rocm': '5.5'},
                {'name': 'RX 7800 XT', 'memory_gb': 16, 'inference': True, 'training': False, 'rocm': '5.5'},
                {'name': 'RX 7700 XT', 'memory_gb': 12, 'inference': True, 'training': False, 'rocm': '5.5'},
                {'name': 'RX 7600 XT', 'memory_gb': 16, 'inference': True, 'training': False, 'rocm': '5.5'},
                {'name': 'RX 7600', 'memory_gb': 8, 'inference': True, 'training': False, 'rocm': '5.5'},
                # RDNA 2 (RX 6000 series)
                {'name': 'RX 6950 XT', 'memory_gb': 16, 'inference': True, 'training': False, 'rocm': '5.0'},
                {'name': 'RX 6900 XT', 'memory_gb': 16, 'inference': True, 'training': False, 'rocm': '5.0'},
                {'name': 'RX 6800 XT', 'memory_gb': 16, 'inference': True, 'training': False, 'rocm': '5.0'},
                {'name': 'RX 6700 XT', 'memory_gb': 12, 'inference': True, 'training': False, 'rocm': '5.0'},
                {'name': 'RX 6600 XT', 'memory_gb': 8, 'inference': True, 'training': False, 'rocm': '5.0'},
                # Legacy Vega
                {'name': 'Radeon VII', 'memory_gb': 16, 'inference': True, 'training': True, 'rocm': '3.0'},
                {'name': 'Vega 64', 'memory_gb': 8, 'inference': True, 'training': False, 'rocm': '2.0'},
                {'name': 'Vega 56', 'memory_gb': 8, 'inference': True, 'training': False, 'rocm': '2.0'},
            ],
        },
    },
    
    # Intel GPUs
    'intel': {
        # Data Center
        'data_center': {
            'models': [
                {'name': 'Data Center GPU Max 1550', 'memory_gb': 128, 'inference': True, 'training': True, 'xmx': True},
                {'name': 'Data Center GPU Max 1100', 'memory_gb': 48, 'inference': True, 'training': True, 'xmx': True},
                {'name': 'Data Center GPU Flex 170', 'memory_gb': 16, 'inference': True, 'training': False, 'xmx': True},
                {'name': 'Data Center GPU Flex 140', 'memory_gb': 12, 'inference': True, 'training': False, 'xmx': True},
            ],
        },
        # Professional / Workstation
        'professional': {
            'models': [
                # Arc Pro Series
                {'name': 'Arc Pro A60', 'memory_gb': 12, 'inference': True, 'training': False, 'xmx': True},
                {'name': 'Arc Pro A50', 'memory_gb': 6, 'inference': True, 'training': False, 'xmx': True},
                {'name': 'Arc Pro A40', 'memory_gb': 6, 'inference': True, 'training': False, 'xmx': True},
                {'name': 'Arc Pro A30M', 'memory_gb': 4, 'inference': True, 'training': False, 'xmx': True},
            ],
        },
        # Consumer
        'consumer': {
            'models': [
                {'name': 'Arc A770', 'memory_gb': 16, 'inference': True, 'training': False, 'xmx': True},
                {'name': 'Arc A750', 'memory_gb': 8, 'inference': True, 'training': False, 'xmx': True},
                {'name': 'Arc A580', 'memory_gb': 8, 'inference': True, 'training': False, 'xmx': True},
                {'name': 'Arc A380', 'memory_gb': 6, 'inference': True, 'training': False, 'xmx': True},
            ],
        },
    },
    
    # Apple Silicon
    'apple': {
        'models': [
            # M3 Series (2023+)
            {'name': 'M3 Max', 'memory_gb': 128, 'gpu_cores': 40, 'inference': True, 'training': True, 'neural_engine': True},
            {'name': 'M3 Pro', 'memory_gb': 36, 'gpu_cores': 18, 'inference': True, 'training': True, 'neural_engine': True},
            {'name': 'M3', 'memory_gb': 24, 'gpu_cores': 10, 'inference': True, 'training': False, 'neural_engine': True},
            # M2 Series (2022)
            {'name': 'M2 Ultra', 'memory_gb': 192, 'gpu_cores': 76, 'inference': True, 'training': True, 'neural_engine': True},
            {'name': 'M2 Max', 'memory_gb': 96, 'gpu_cores': 38, 'inference': True, 'training': True, 'neural_engine': True},
            {'name': 'M2 Pro', 'memory_gb': 32, 'gpu_cores': 19, 'inference': True, 'training': True, 'neural_engine': True},
            {'name': 'M2', 'memory_gb': 24, 'gpu_cores': 10, 'inference': True, 'training': False, 'neural_engine': True},
            # M1 Series (2020)
            {'name': 'M1 Ultra', 'memory_gb': 128, 'gpu_cores': 64, 'inference': True, 'training': True, 'neural_engine': True},
            {'name': 'M1 Max', 'memory_gb': 64, 'gpu_cores': 32, 'inference': True, 'training': True, 'neural_engine': True},
            {'name': 'M1 Pro', 'memory_gb': 32, 'gpu_cores': 16, 'inference': True, 'training': True, 'neural_engine': True},
            {'name': 'M1', 'memory_gb': 16, 'gpu_cores': 8, 'inference': True, 'training': False, 'neural_engine': True},
        ],
    },
}

# Minimum requirements for workload types
GPU_WORKLOAD_REQUIREMENTS = {
    'inference': {
        'min_memory_gb': 4,
        'preferred_memory_gb': 8,
    },
    'training': {
        'min_memory_gb': 8,
        'preferred_memory_gb': 16,
    },
}
