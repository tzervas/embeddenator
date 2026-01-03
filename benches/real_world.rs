//! Real-World Data Benchmarks
//!
//! This benchmark suite tests VSA operations with realistic data types:
//! - Images (PNG, JPEG)
//! - Video frames (extracted from clips)
//! - Audio samples
//! - Text documents
//! - Binary blobs (executables, archives)
//! - Synthetic render tasks (gradients, noise, patterns)
//!
//! To run with test data:
//! ```bash
//! # Download sample data first
//! ./scripts/fetch_benchmark_data.sh
//!
//! # Run benchmarks
//! cargo bench --bench real_world
//! ```

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use embeddenator::{BitslicedTritVec, ReversibleVSAConfig, SparseVec, TernaryInvertedIndex, DIM};
use std::fs;
use std::time::Duration;

// ============================================================================
// TEST DATA GENERATION
// ============================================================================

/// Generate synthetic image data (gradient pattern)
fn generate_gradient_image(width: usize, height: usize) -> Vec<u8> {
    let mut data = Vec::with_capacity(width * height * 3);
    for y in 0..height {
        for x in 0..width {
            let r = ((x * 255) / width) as u8;
            let g = ((y * 255) / height) as u8;
            let b = (((x + y) * 128) / (width + height)) as u8;
            data.push(r);
            data.push(g);
            data.push(b);
        }
    }
    data
}

/// Generate synthetic noise pattern (pseudo-random)
fn generate_noise_pattern(size: usize, seed: u64) -> Vec<u8> {
    let mut data = Vec::with_capacity(size);
    let mut state = seed;
    for _ in 0..size {
        // Simple LCG for reproducible pseudo-random data
        state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
        data.push((state >> 56) as u8);
    }
    data
}

/// Generate synthetic video frame sequence (motion gradient)
fn generate_video_frames(width: usize, height: usize, num_frames: usize) -> Vec<Vec<u8>> {
    (0..num_frames)
        .map(|frame| {
            let offset = frame * 10;
            let mut data = Vec::with_capacity(width * height * 3);
            for y in 0..height {
                for x in 0..width {
                    let r = (((x + offset) * 255) / width) as u8;
                    let g = (((y + offset) * 255) / height) as u8;
                    let b = ((frame * 17) % 256) as u8;
                    data.push(r);
                    data.push(g);
                    data.push(b);
                }
            }
            data
        })
        .collect()
}

/// Generate synthetic audio waveform (sine wave approximation)
fn generate_audio_samples(num_samples: usize, frequency_hz: f32, sample_rate: u32) -> Vec<u8> {
    let mut data = Vec::with_capacity(num_samples * 2); // 16-bit samples
    for i in 0..num_samples {
        let t = i as f32 / sample_rate as f32;
        let sample = ((t * frequency_hz * std::f32::consts::TAU).sin() * 32767.0) as i16;
        data.push((sample & 0xFF) as u8);
        data.push((sample >> 8) as u8);
    }
    data
}

/// Generate text document with realistic structure
fn generate_text_document(paragraphs: usize, words_per_para: usize) -> Vec<u8> {
    let words = [
        "the", "quick", "brown", "fox", "jumps", "over", "lazy", "dog",
        "embeddenator", "holographic", "computing", "vector", "symbolic",
        "architecture", "sparse", "ternary", "encoding", "retrieval",
        "dimension", "binding", "bundling", "permutation", "cosine",
        "similarity", "reconstruction", "lossless", "compression",
    ];
    
    let mut text = String::new();
    let mut word_idx = 0;
    
    for p in 0..paragraphs {
        for w in 0..words_per_para {
            if w > 0 { text.push(' '); }
            text.push_str(words[(word_idx + p * 7 + w * 3) % words.len()]);
            word_idx = (word_idx + 1) % words.len();
        }
        text.push_str(".\n\n");
    }
    
    text.into_bytes()
}

/// Generate synthetic binary blob (executable-like pattern)
fn generate_binary_blob(size: usize) -> Vec<u8> {
    let mut data = Vec::with_capacity(size);
    
    // ELF-like header
    data.extend_from_slice(&[0x7f, b'E', b'L', b'F']);
    data.extend_from_slice(&[2, 1, 1, 0]); // 64-bit, little endian, v1, SYSV
    data.extend_from_slice(&[0; 8]); // padding
    
    // Fill with mix of patterns
    let mut offset = data.len();
    while offset < size {
        let pattern_type = (offset / 256) % 4;
        match pattern_type {
            0 => data.push(0x90), // NOP slide
            1 => data.push((offset & 0xFF) as u8), // Sequential
            2 => data.push(0x00), // Zero fill
            _ => data.push(0xCC), // INT3
        }
        offset += 1;
    }
    
    data.truncate(size);
    data
}

/// Load real data from benchmark_data directory if available
fn load_real_data(filename: &str) -> Option<Vec<u8>> {
    let paths = [
        format!("benchmark_data/{}", filename),
        format!("benches/benchmark_data/{}", filename),
        format!("../benchmark_data/{}", filename),
    ];
    
    for path in &paths {
        if let Ok(data) = fs::read(path) {
            return Some(data);
        }
    }
    None
}

// ============================================================================
// IMAGE BENCHMARKS
// ============================================================================

fn bench_image_encoding(c: &mut Criterion) {
    let mut group = c.benchmark_group("image_encoding");
    group.measurement_time(Duration::from_secs(10));
    
    let config = ReversibleVSAConfig::default();
    
    // Test various image sizes
    let sizes = [
        ("thumbnail_64x64", 64, 64),
        ("small_256x256", 256, 256),
        ("medium_512x512", 512, 512),
        ("large_1024x1024", 1024, 1024),
    ];
    
    for (name, width, height) in sizes {
        let image_data = generate_gradient_image(width, height);
        let data_size = image_data.len();
        
        group.throughput(Throughput::Bytes(data_size as u64));
        
        group.bench_with_input(
            BenchmarkId::new("encode_gradient", name),
            &image_data,
            |bencher, data| {
                bencher.iter(|| {
                    let vec = SparseVec::encode_data(black_box(data), black_box(&config), Some("/bench/image"));
                    black_box(vec)
                })
            },
        );
        
        // Also test decode roundtrip
        let encoded = SparseVec::encode_data(&image_data, &config, Some("/bench/image"));
        group.bench_with_input(
            BenchmarkId::new("decode_gradient", name),
            &encoded,
            |bencher, encoded| {
                bencher.iter(|| {
                    let decoded = black_box(encoded).decode_data(
                        black_box(&config),
                        Some("/bench/image"),
                        data_size,
                    );
                    black_box(decoded)
                })
            },
        );
    }
    
    // Test with noise pattern (high entropy)
    let noise_data = generate_noise_pattern(256 * 256 * 3, 0xDEADBEEF);
    group.throughput(Throughput::Bytes(noise_data.len() as u64));
    
    group.bench_with_input(
        BenchmarkId::new("encode_noise", "256x256"),
        &noise_data,
        |bencher, data| {
            bencher.iter(|| {
                let vec = SparseVec::encode_data(black_box(data), black_box(&config), Some("/bench/noise"));
                black_box(vec)
            })
        },
    );
    
    // Try loading real image data
    if let Some(real_image) = load_real_data("sample.png").or_else(|| load_real_data("sample.jpg")) {
        group.throughput(Throughput::Bytes(real_image.len() as u64));
        
        group.bench_with_input(
            BenchmarkId::new("encode_real_image", format!("{}KB", real_image.len() / 1024)),
            &real_image,
            |bencher, data| {
                bencher.iter(|| {
                    let vec = SparseVec::encode_data(black_box(data), black_box(&config), Some("/bench/real_img"));
                    black_box(vec)
                })
            },
        );
    }
    
    group.finish();
}

// ============================================================================
// VIDEO FRAME BENCHMARKS
// ============================================================================

fn bench_video_frames(c: &mut Criterion) {
    let mut group = c.benchmark_group("video_frames");
    group.measurement_time(Duration::from_secs(15));
    
    let config = ReversibleVSAConfig::default();
    
    // Simulate video processing at different resolutions
    let resolutions = [
        ("480p", 854, 480, 30),   // 30 frames @ 480p
        ("720p", 1280, 720, 24),  // 24 frames @ 720p
        ("1080p", 1920, 1080, 10), // 10 frames @ 1080p
    ];
    
    for (name, width, height, num_frames) in resolutions {
        let frames = generate_video_frames(width, height, num_frames);
        let total_bytes: usize = frames.iter().map(|f| f.len()).sum();
        
        group.throughput(Throughput::Bytes(total_bytes as u64));
        
        // Encode all frames
        group.bench_with_input(
            BenchmarkId::new("encode_sequence", name),
            &frames,
            |bencher, frames| {
                bencher.iter(|| {
                    let encoded: Vec<_> = frames.iter()
                        .enumerate()
                        .map(|(i, frame)| {
                            SparseVec::encode_data(
                                black_box(frame),
                                black_box(&config),
                                Some(&format!("/video/frame_{}", i)),
                            )
                        })
                        .collect();
                    black_box(encoded)
                })
            },
        );
        
        // Test frame similarity detection (motion estimation proxy)
        let encoded_frames: Vec<_> = frames.iter()
            .enumerate()
            .map(|(i, frame)| {
                SparseVec::encode_data(frame, &config, Some(&format!("/video/frame_{}", i)))
            })
            .collect();
        
        group.bench_with_input(
            BenchmarkId::new("frame_similarity", name),
            &encoded_frames,
            |bencher, frames| {
                bencher.iter(|| {
                    let mut similarities = Vec::with_capacity(frames.len() - 1);
                    for i in 0..frames.len() - 1 {
                        similarities.push(black_box(&frames[i]).cosine(black_box(&frames[i + 1])));
                    }
                    black_box(similarities)
                })
            },
        );
        
        // Test bundling frames (temporal superposition)
        group.bench_with_input(
            BenchmarkId::new("bundle_sequence", name),
            &encoded_frames,
            |bencher, frames| {
                bencher.iter(|| {
                    let bundled = SparseVec::bundle_sum_many(black_box(frames.iter()));
                    black_box(bundled)
                })
            },
        );
    }
    
    group.finish();
}

// ============================================================================
// AUDIO BENCHMARKS
// ============================================================================

fn bench_audio_encoding(c: &mut Criterion) {
    let mut group = c.benchmark_group("audio_encoding");
    group.measurement_time(Duration::from_secs(10));
    
    let config = ReversibleVSAConfig::default();
    
    // Test various audio durations (44.1kHz sample rate, 16-bit)
    let sample_rate: u32 = 44100;
    let durations_ms = [100, 500, 1000, 5000]; // 0.1s to 5s
    
    for duration_ms in durations_ms {
        let num_samples = ((sample_rate as usize) * duration_ms) / 1000;
        let audio_data = generate_audio_samples(num_samples, 440.0, sample_rate); // A4 note
        
        let name = format!("{}ms", duration_ms);
        group.throughput(Throughput::Bytes(audio_data.len() as u64));
        
        group.bench_with_input(
            BenchmarkId::new("encode_sine", &name),
            &audio_data,
            |bencher, data| {
                bencher.iter(|| {
                    let vec = SparseVec::encode_data(black_box(data), black_box(&config), Some("/audio/sample"));
                    black_box(vec)
                })
            },
        );
    }
    
    // Test audio fingerprinting scenario (compare audio segments)
    let reference = generate_audio_samples(44100, 440.0, sample_rate); // 1 second
    let similar = generate_audio_samples(44100, 442.0, sample_rate); // Slightly detuned
    let different = generate_audio_samples(44100, 880.0, sample_rate); // Octave up
    
    let ref_vec = SparseVec::encode_data(&reference, &config, Some("/audio/ref"));
    let sim_vec = SparseVec::encode_data(&similar, &config, Some("/audio/sim"));
    let diff_vec = SparseVec::encode_data(&different, &config, Some("/audio/diff"));
    
    group.bench_function("fingerprint_compare", |bencher| {
        bencher.iter(|| {
            let sim1 = black_box(&ref_vec).cosine(black_box(&sim_vec));
            let sim2 = black_box(&ref_vec).cosine(black_box(&diff_vec));
            black_box((sim1, sim2))
        })
    });
    
    group.finish();
}

// ============================================================================
// DOCUMENT BENCHMARKS
// ============================================================================

fn bench_document_encoding(c: &mut Criterion) {
    let mut group = c.benchmark_group("document_encoding");
    group.measurement_time(Duration::from_secs(10));
    
    let config = ReversibleVSAConfig::default();
    
    // Test various document sizes
    let doc_sizes = [
        ("small_1KB", 10, 20),    // ~1KB
        ("medium_10KB", 100, 20), // ~10KB
        ("large_100KB", 500, 40), // ~100KB
    ];
    
    for (name, paragraphs, words) in doc_sizes {
        let doc_data = generate_text_document(paragraphs, words);
        
        group.throughput(Throughput::Bytes(doc_data.len() as u64));
        
        group.bench_with_input(
            BenchmarkId::new("encode_text", name),
            &doc_data,
            |bencher, data| {
                bencher.iter(|| {
                    let vec = SparseVec::encode_data(black_box(data), black_box(&config), Some("/docs/sample"));
                    black_box(vec)
                })
            },
        );
        
        // Test decode roundtrip
        let encoded = SparseVec::encode_data(&doc_data, &config, Some("/docs/sample"));
        let original_size = doc_data.len();
        
        group.bench_with_input(
            BenchmarkId::new("decode_text", name),
            &encoded,
            |bencher, encoded| {
                bencher.iter(|| {
                    let decoded = black_box(encoded).decode_data(
                        black_box(&config),
                        Some("/docs/sample"),
                        original_size,
                    );
                    black_box(decoded)
                })
            },
        );
    }
    
    // Document retrieval benchmark
    let num_docs = 1000;
    let docs: Vec<_> = (0..num_docs)
        .map(|i| {
            let doc = generate_text_document(5 + (i % 10), 15 + (i % 10));
            SparseVec::encode_data(&doc, &config, Some(&format!("/docs/{}", i)))
        })
        .collect();
    
    // Build retrieval index
    let mut index = TernaryInvertedIndex::new();
    for (i, doc) in docs.iter().enumerate() {
        index.add(i, doc);
    }
    index.finalize();
    
    let query = SparseVec::encode_data(b"holographic computing vector symbolic", &config, Some("/query"));
    
    group.bench_function("document_retrieval_top20", |bencher| {
        bencher.iter(|| {
            let results = index.query_top_k(black_box(&query), 20);
            black_box(results)
        })
    });
    
    group.finish();
}

// ============================================================================
// BINARY BLOB BENCHMARKS
// ============================================================================

fn bench_binary_encoding(c: &mut Criterion) {
    let mut group = c.benchmark_group("binary_encoding");
    group.measurement_time(Duration::from_secs(10));
    
    let config = ReversibleVSAConfig::default();
    
    // Test various binary sizes
    let blob_sizes = [
        ("4KB", 4 * 1024),
        ("64KB", 64 * 1024),
        ("256KB", 256 * 1024),
        ("1MB", 1024 * 1024),
    ];
    
    for (name, size) in blob_sizes {
        let blob_data = generate_binary_blob(size);
        
        group.throughput(Throughput::Bytes(blob_data.len() as u64));
        
        group.bench_with_input(
            BenchmarkId::new("encode_binary", name),
            &blob_data,
            |bencher, data| {
                bencher.iter(|| {
                    let vec = SparseVec::encode_data(black_box(data), black_box(&config), Some("/bin/blob"));
                    black_box(vec)
                })
            },
        );
        
        // Decode roundtrip
        let encoded = SparseVec::encode_data(&blob_data, &config, Some("/bin/blob"));
        let original_size = blob_data.len();
        
        group.bench_with_input(
            BenchmarkId::new("decode_binary", name),
            &encoded,
            |bencher, encoded| {
                bencher.iter(|| {
                    let decoded = black_box(encoded).decode_data(
                        black_box(&config),
                        Some("/bin/blob"),
                        original_size,
                    );
                    black_box(decoded)
                })
            },
        );
    }
    
    group.finish();
}

// ============================================================================
// RENDER TASK BENCHMARKS
// ============================================================================

/// Simulates caching/retrieving render task outputs
fn bench_render_tasks(c: &mut Criterion) {
    let mut group = c.benchmark_group("render_tasks");
    group.measurement_time(Duration::from_secs(15));
    
    let config = ReversibleVSAConfig::default();
    
    // Simulate various render outputs
    let render_outputs = [
        ("tile_256x256_rgb", generate_gradient_image(256, 256)),
        ("tile_512x512_rgb", generate_gradient_image(512, 512)),
        ("noise_tile_256", generate_noise_pattern(256 * 256 * 4, 0xCAFEBABE)), // RGBA
        ("depth_buffer_512", generate_noise_pattern(512 * 512 * 4, 0xDEADC0DE)), // float32 depth
    ];
    
    // Encode render outputs
    for (name, data) in &render_outputs {
        group.throughput(Throughput::Bytes(data.len() as u64));
        
        group.bench_with_input(
            BenchmarkId::new("encode_render", *name),
            data,
            |bencher, data| {
                bencher.iter(|| {
                    let vec = SparseVec::encode_data(black_box(data), black_box(&config), Some("/render/output"));
                    black_box(vec)
                })
            },
        );
    }
    
    // Simulate render cache scenario: store multiple tiles, query similar ones
    let num_tiles = 100;
    let tile_size = 128 * 128 * 3;
    let tiles: Vec<_> = (0..num_tiles)
        .map(|i| {
            let data = generate_noise_pattern(tile_size, i as u64 * 12345);
            SparseVec::encode_data(&data, &config, Some(&format!("/render/tile_{}", i)))
        })
        .collect();
    
    // Build tile cache index
    let mut cache_index = TernaryInvertedIndex::new();
    for (i, tile) in tiles.iter().enumerate() {
        cache_index.add(i, tile);
    }
    cache_index.finalize();
    
    // Query similar tiles
    let query_tile = generate_noise_pattern(tile_size, 50 * 12345 + 100); // Slightly modified tile 50
    let query_vec = SparseVec::encode_data(&query_tile, &config, Some("/render/query"));
    
    group.bench_function("cache_lookup_similar_tiles", |bencher| {
        bencher.iter(|| {
            let similar = cache_index.query_top_k(black_box(&query_vec), 5);
            black_box(similar)
        })
    });
    
    // Batch encoding scenario (render farm output)
    let batch_size = 10;
    let batch_data: Vec<_> = (0..batch_size)
        .map(|i| generate_noise_pattern(tile_size, i as u64 * 99999))
        .collect();
    
    let total_batch_bytes: usize = batch_data.iter().map(|d| d.len()).sum();
    group.throughput(Throughput::Bytes(total_batch_bytes as u64));
    
    group.bench_with_input(
        BenchmarkId::new("batch_encode", format!("{}_tiles", batch_size)),
        &batch_data,
        |bencher, batch| {
            bencher.iter(|| {
                let encoded: Vec<_> = batch.iter()
                    .enumerate()
                    .map(|(i, data)| {
                        SparseVec::encode_data(
                            black_box(data),
                            black_box(&config),
                            Some(&format!("/render/batch_{}", i)),
                        )
                    })
                    .collect();
                black_box(encoded)
            })
        },
    );
    
    group.finish();
}

// ============================================================================
// MIXED WORKLOAD BENCHMARKS
// ============================================================================

/// Simulates realistic mixed workload (typical application usage)
fn bench_mixed_workload(c: &mut Criterion) {
    let mut group = c.benchmark_group("mixed_workload");
    group.measurement_time(Duration::from_secs(20));
    
    let config = ReversibleVSAConfig::default();
    
    // Generate diverse data set
    let image = generate_gradient_image(256, 256);
    let audio = generate_audio_samples(22050, 440.0, 44100); // 0.5s audio
    let text = generate_text_document(20, 30);
    let binary = generate_binary_blob(32 * 1024);
    
    // Pre-encode for similarity tests
    let image_vec = SparseVec::encode_data(&image, &config, Some("/mixed/image"));
    let audio_vec = SparseVec::encode_data(&audio, &config, Some("/mixed/audio"));
    let text_vec = SparseVec::encode_data(&text, &config, Some("/mixed/text"));
    let binary_vec = SparseVec::encode_data(&binary, &config, Some("/mixed/binary"));
    
    // Encode all types
    group.bench_function("encode_all_types", |bencher| {
        bencher.iter(|| {
            let v1 = SparseVec::encode_data(black_box(&image), black_box(&config), Some("/m/img"));
            let v2 = SparseVec::encode_data(black_box(&audio), black_box(&config), Some("/m/aud"));
            let v3 = SparseVec::encode_data(black_box(&text), black_box(&config), Some("/m/txt"));
            let v4 = SparseVec::encode_data(black_box(&binary), black_box(&config), Some("/m/bin"));
            black_box((v1, v2, v3, v4))
        })
    });
    
    // Cross-type similarity (should be low)
    group.bench_function("cross_type_similarity", |bencher| {
        bencher.iter(|| {
            let s1 = black_box(&image_vec).cosine(black_box(&audio_vec));
            let s2 = black_box(&image_vec).cosine(black_box(&text_vec));
            let s3 = black_box(&audio_vec).cosine(black_box(&binary_vec));
            let s4 = black_box(&text_vec).cosine(black_box(&binary_vec));
            black_box((s1, s2, s3, s4))
        })
    });
    
    // Bundle all types (semantic superposition)
    group.bench_function("bundle_all_types", |bencher| {
        bencher.iter(|| {
            let bundled = SparseVec::bundle_sum_many([
                black_box(&image_vec),
                black_box(&audio_vec),
                black_box(&text_vec),
                black_box(&binary_vec),
            ]);
            black_box(bundled)
        })
    });
    
    // Bitsliced conversion and operations
    let image_bs = BitslicedTritVec::from_sparse(&image_vec, DIM);
    let text_bs = BitslicedTritVec::from_sparse(&text_vec, DIM);
    
    group.bench_function("bitsliced_mixed_ops", |bencher| {
        bencher.iter(|| {
            let bound = black_box(&image_bs).bind(black_box(&text_bs));
            let bundled = black_box(&image_bs).bundle(black_box(&text_bs));
            let cosine = black_box(&image_bs).cosine(black_box(&text_bs));
            black_box((bound, bundled, cosine))
        })
    });
    
    group.finish();
}

// ============================================================================
// STREAMING DATA BENCHMARKS
// ============================================================================

/// Benchmarks for streaming/chunked data processing
fn bench_streaming(c: &mut Criterion) {
    let mut group = c.benchmark_group("streaming");
    group.measurement_time(Duration::from_secs(15));
    
    let config = ReversibleVSAConfig::default();
    
    // Simulate streaming video chunks
    let chunk_size = 64 * 1024; // 64KB chunks
    let num_chunks = 50;
    let chunks: Vec<_> = (0..num_chunks)
        .map(|i| generate_noise_pattern(chunk_size, i as u64 * 7777))
        .collect();
    
    let total_bytes = chunk_size * num_chunks;
    group.throughput(Throughput::Bytes(total_bytes as u64));
    
    // Sequential chunk encoding
    group.bench_with_input(
        BenchmarkId::new("encode_chunks_sequential", format!("{}x64KB", num_chunks)),
        &chunks,
        |bencher, chunks| {
            bencher.iter(|| {
                let encoded: Vec<_> = chunks.iter()
                    .enumerate()
                    .map(|(i, chunk)| {
                        SparseVec::encode_data(
                            black_box(chunk),
                            black_box(&config),
                            Some(&format!("/stream/chunk_{}", i)),
                        )
                    })
                    .collect();
                black_box(encoded)
            })
        },
    );
    
    // Pre-encode for rolling window tests
    let encoded_chunks: Vec<_> = chunks.iter()
        .enumerate()
        .map(|(i, chunk)| {
            SparseVec::encode_data(chunk, &config, Some(&format!("/stream/chunk_{}", i)))
        })
        .collect();
    
    // Rolling window bundle (sliding window aggregation)
    let window_size = 5;
    group.bench_function("rolling_window_bundle", |bencher| {
        bencher.iter(|| {
            let windows: Vec<_> = encoded_chunks.windows(window_size)
                .map(|window| SparseVec::bundle_sum_many(window.iter()))
                .collect();
            black_box(windows)
        })
    });
    
    // Incremental similarity checking
    group.bench_function("incremental_similarity", |bencher| {
        bencher.iter(|| {
            let mut similarities = Vec::with_capacity(num_chunks - 1);
            for i in 0..num_chunks - 1 {
                similarities.push(
                    black_box(&encoded_chunks[i]).cosine(black_box(&encoded_chunks[i + 1]))
                );
            }
            black_box(similarities)
        })
    });
    
    group.finish();
}

// ============================================================================
// CRITERION SETUP
// ============================================================================

criterion_group!(
    name = real_world_benches;
    config = Criterion::default()
        .sample_size(50)
        .warm_up_time(Duration::from_secs(3));
    targets =
        bench_image_encoding,
        bench_video_frames,
        bench_audio_encoding,
        bench_document_encoding,
        bench_binary_encoding,
        bench_render_tasks,
        bench_mixed_workload,
        bench_streaming
);

criterion_main!(real_world_benches);
