//! GPU benchmark - test scaling with different batch sizes

#[cfg(feature = "cuda")]
use embeddenator_vsa::{GpuBackend, GpuConfig, SparseVec};

fn main() {
    #[cfg(feature = "cuda")]
    {
        println!("=== GPU Scaling Benchmark ===\n");

        let config = GpuConfig::default();
        let gpu = GpuBackend::new(config).expect("GPU init failed");

        println!("GPU: {:?}", gpu.architecture());
        let mem = gpu.memory_config();
        println!("Memory: {:.1}GB safe limit\n", mem.safe_limit as f64 / 1e9);

        // Test different batch sizes
        for batch_size in [100, 1000, 5000, 10000, 50000] {
            let query = SparseVec::random();
            let candidates: Vec<SparseVec> = (0..batch_size).map(|_| SparseVec::random()).collect();

            // GPU timing
            let gpu_start = std::time::Instant::now();
            let _ = gpu.batch_cosine(&query, &candidates).unwrap();
            let gpu_time = gpu_start.elapsed();

            // CPU timing
            let cpu_start = std::time::Instant::now();
            let _: Vec<f64> = candidates.iter().map(|c| query.cosine(c)).collect();
            let cpu_time = cpu_start.elapsed();

            let speedup = cpu_time.as_secs_f64() / gpu_time.as_secs_f64();
            println!(
                "Batch {:6}: GPU {:8.2}ms, CPU {:8.2}ms, Speedup: {:5.1}x",
                batch_size,
                gpu_time.as_secs_f64() * 1000.0,
                cpu_time.as_secs_f64() * 1000.0,
                speedup
            );
        }
    }

    #[cfg(not(feature = "cuda"))]
    println!("CUDA feature not enabled");
}
