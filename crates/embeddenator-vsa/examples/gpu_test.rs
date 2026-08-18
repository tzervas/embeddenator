//! GPU backend test - exercises dynamic memory detection and batch cosine

#[cfg(feature = "cuda")]
use embeddenator_vsa::{GpuBackend, GpuConfig, SparseVec};

fn main() {
    #[cfg(feature = "cuda")]
    {
        println!("=== GPU Backend Test ===\n");

        // Create GPU backend with auto-detection
        let config = GpuConfig::default();

        match GpuBackend::new(config) {
            Ok(gpu) => {
                println!("GPU Backend initialized successfully!");
                println!("  Architecture: {:?}", gpu.architecture());

                let mem = gpu.memory_config();
                println!("  Total Memory: {:.2} GB", mem.total_memory as f64 / 1e9);
                println!("  Free Memory:  {:.2} GB", mem.free_memory as f64 / 1e9);
                println!("  Safe Limit:   {:.2} GB", mem.safe_limit as f64 / 1e9);
                println!("  Headroom:     {:.0}%", mem.headroom_percent * 100.0);

                // Generate test vectors
                let query = SparseVec::random();
                let candidates: Vec<SparseVec> = (0..1000).map(|_| SparseVec::random()).collect();

                println!("\n  Query pos/neg: {}/{}", query.pos.len(), query.neg.len());
                println!("  Candidates: {}", candidates.len());

                // Run batch cosine
                println!("\n  Running batch cosine on GPU...");
                let start = std::time::Instant::now();
                let similarities = gpu
                    .batch_cosine(&query, &candidates)
                    .expect("batch_cosine failed");
                let elapsed = start.elapsed();

                println!("  Completed in {:?}", elapsed);
                println!("  Results: {} similarities", similarities.len());

                // Find max similarity
                let max_sim = similarities
                    .iter()
                    .cloned()
                    .max_by(|a, b| a.partial_cmp(b).unwrap())
                    .unwrap_or(0.0);
                println!("  Max similarity: {:.4}", max_sim);

                // Verify against CPU
                println!("\n  Verifying against CPU...");
                let cpu_start = std::time::Instant::now();
                let cpu_sims: Vec<f64> = candidates.iter().map(|c| query.cosine(c)).collect();
                let cpu_elapsed = cpu_start.elapsed();
                println!("  CPU completed in {:?}", cpu_elapsed);

                // Check accuracy
                let mut max_diff = 0.0f64;
                for (gpu_sim, cpu_sim) in similarities.iter().zip(cpu_sims.iter()) {
                    let diff = (gpu_sim - cpu_sim).abs();
                    if diff > max_diff {
                        max_diff = diff;
                    }
                }
                println!("  Max GPU/CPU difference: {:.6}", max_diff);

                if max_diff < 0.001 {
                    println!("\n✓ GPU accuracy verified!");
                } else {
                    println!("\n✗ GPU accuracy issue: max diff {:.6}", max_diff);
                }

                let speedup = cpu_elapsed.as_secs_f64() / elapsed.as_secs_f64();
                println!("  Speedup: {:.1}x", speedup);
            }
            Err(e) => {
                eprintln!("Failed to initialize GPU: {}", e);
            }
        }
    }

    #[cfg(not(feature = "cuda"))]
    {
        println!("CUDA feature not enabled. Build with --features cuda");
    }
}
