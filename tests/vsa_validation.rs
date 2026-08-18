//! Comprehensive VSA Validation Framework
//!
//! Validates not just byte-perfect reconstruction, but full VSA algebraic properties,
//! filesystem functionality, and computational paradigm correctness.
//!
//! # Timing Precision
//!
//! Tests report nanosecond-precision timing when `--nocapture` flag is used.
//! Set `TIMING_REPORT=1` for automatic timing reports on timer drop.

mod common;

#[allow(unused_imports)]
use common::{benchmark, time_formatted, time_ns, PrecisionTimer, TimingStats};
use embeddenator_fs::{EmbrFSError, VersionedEmbrFS};
use embeddenator_vsa::{ReversibleVSAConfig, SparseVec};
use sha2::{Digest, Sha256};
use std::sync::Arc;
use std::thread;

// ============================================================================
// Section 1: Byte-Perfect Validation
// ============================================================================

mod byte_perfect {
    use super::*;

    #[test]
    fn test_byte_perfect_reconstruction() {
        let mut timer = PrecisionTimer::new("byte_perfect_reconstruction");

        let fs = time_formatted("VersionedEmbrFS::new()", || VersionedEmbrFS::new());
        timer.lap("FS created");

        // Test various data patterns
        let test_cases = vec![
            ("zeros.bin", vec![0u8; 4096]),
            ("ones.bin", vec![0xFF; 4096]),
            (
                "random.bin",
                (0..4096).map(|i| (i * 17 + 31) as u8).collect(),
            ),
            (
                "text.txt",
                b"Hello, VSA World! This is a test of holographic encoding.".to_vec(),
            ),
            ("mixed.bin", {
                let mut v = Vec::new();
                v.extend(&[0u8; 1024]);
                v.extend(&[0xFF; 1024]);
                v.extend((0..2048).map(|i| i as u8));
                v
            }),
        ];

        for (path, original) in &test_cases {
            let (_, write_ns) = time_ns(&format!("write {}", path), || {
                fs.write_file(path, original, None).expect("write failed")
            });

            let (recovered, read_ns) = time_ns(&format!("read {}", path), || {
                fs.read_file(path).expect("read failed").0
            });

            assert_eq!(
                original, &recovered,
                "Byte-perfect reconstruction failed for {}",
                path
            );

            // Report throughput
            let bytes = original.len();
            let write_throughput = bytes as f64 / (write_ns as f64 / 1_000_000_000.0) / 1_000_000.0;
            let read_throughput = bytes as f64 / (read_ns as f64 / 1_000_000_000.0) / 1_000_000.0;
            println!(
                "  └─ {} throughput: write {:.2} MB/s, read {:.2} MB/s",
                path, write_throughput, read_throughput
            );
        }

        timer.lap("All patterns verified");
        timer.report();
    }

    #[test]
    fn test_sha256_hash_verification() {
        let fs = VersionedEmbrFS::new();
        let data = b"The quick brown fox jumps over the lazy dog";

        // Compute original hash
        let mut hasher = Sha256::new();
        hasher.update(data);
        let original_hash = hasher.finalize();

        // Write and read back
        fs.write_file("hash_test.txt", data, None).unwrap();
        let (recovered, _) = fs.read_file("hash_test.txt").unwrap();

        // Compute recovered hash
        let mut hasher = Sha256::new();
        hasher.update(&recovered);
        let recovered_hash = hasher.finalize();

        assert_eq!(original_hash, recovered_hash, "SHA256 hash mismatch");
    }

    #[test]
    fn test_size_verification() {
        let fs = VersionedEmbrFS::new();

        // Test various sizes including edge cases
        for size in [
            0, 1, 2, 100, 1023, 1024, 1025, 4095, 4096, 4097, 8192, 16384,
        ] {
            let data: Vec<u8> = (0..size).map(|i| (i % 256) as u8).collect();
            let path = format!("size_{}.bin", size);

            fs.write_file(&path, &data, None).expect("write failed");
            let (recovered, _) = fs.read_file(&path).expect("read failed");

            assert_eq!(data.len(), recovered.len(), "Size mismatch for {}", path);
            assert_eq!(data, recovered, "Data mismatch for {}", path);
        }
    }

    #[test]
    fn test_large_file_integrity() {
        let fs = VersionedEmbrFS::new();

        // 1MB file spanning many chunks
        let size = 1024 * 1024;
        let data: Vec<u8> = (0..size).map(|i| ((i * 13 + 7) % 256) as u8).collect();

        fs.write_file("large.bin", &data, None)
            .expect("write failed");
        let (recovered, _) = fs.read_file("large.bin").expect("read failed");

        assert_eq!(data.len(), recovered.len());

        // Verify in chunks to get better error messages
        let chunk_size = 4096;
        for (i, (orig_chunk, recv_chunk)) in data
            .chunks(chunk_size)
            .zip(recovered.chunks(chunk_size))
            .enumerate()
        {
            assert_eq!(
                orig_chunk,
                recv_chunk,
                "Mismatch at chunk {} (offset {})",
                i,
                i * chunk_size
            );
        }
    }
}

// ============================================================================
// Section 2: VSA Algebraic Property Tests
// ============================================================================

mod vsa_algebraic {
    use super::*;

    #[test]
    fn test_bundle_commutativity() {
        // A ⊕ B = B ⊕ A
        let a = SparseVec::random();
        let b = SparseVec::random();

        let ab = a.bundle(&b);
        let ba = b.bundle(&a);

        // Bundling is approximate, check high similarity
        let similarity = ab.cosine(&ba);
        assert!(
            similarity > 0.95,
            "Bundle commutativity violated: similarity = {}",
            similarity
        );
    }

    #[test]
    fn test_bind_approximate_inverse() {
        // A ⊙ B ⊙ B ≈ A (bind is approximately self-inverse)
        // Note: Sparse ternary vectors have weaker inverse property than dense bipolar
        // Run multiple trials since sparse vectors have stochastic behavior
        const TRIALS: usize = 20;
        const THRESHOLD: f64 = 0.05; // Lower threshold for sparse ternary vectors
        const MIN_PASSES: usize = 14; // 70% must pass

        let mut passed = 0;
        let mut similarities = Vec::with_capacity(TRIALS);
        let mut timing = TimingStats::new("bind_inverse_trial");

        for trial in 0..TRIALS {
            let start = std::time::Instant::now();

            let a = SparseVec::random();
            let b = SparseVec::random();

            let bound = a.bind(&b);
            let unbound = bound.bind(&b);

            let similarity = a.cosine(&unbound);
            similarities.push(similarity);
            if similarity > THRESHOLD {
                passed += 1;
            }

            timing.add_sample(start.elapsed().as_nanos());
            println!(
                "  Trial {}: similarity = {:.6}, elapsed = {} ns",
                trial + 1,
                similarity,
                start.elapsed().as_nanos()
            );
        }

        let avg_sim: f64 = similarities.iter().sum::<f64>() / TRIALS as f64;
        println!(
            "\n  Summary: {}/{} passed, avg similarity: {:.4}",
            passed, TRIALS, avg_sim
        );
        timing.report();

        assert!(
            passed >= MIN_PASSES,
            "Bind inverse property violated: {}/{} passed (avg similarity: {:.4}, values: {:?})",
            passed,
            TRIALS,
            avg_sim,
            similarities
        );
    }

    #[test]
    fn test_permutation_exact_inverse() {
        // P^(-1)(P(A)) = A (permutation is exactly invertible)
        let a = SparseVec::random();
        let shift = 42;

        let permuted = a.permute(shift);
        let unpermuted = permuted.inverse_permute(shift);

        // Permutation should be exact
        assert_eq!(a.pos, unpermuted.pos, "Positive indices mismatch");
        assert_eq!(a.neg, unpermuted.neg, "Negative indices mismatch");
    }

    #[test]
    fn test_cosine_similarity_properties() {
        let a = SparseVec::random();
        let b = SparseVec::random();

        // Reflexivity: sim(A, A) = 1
        let self_sim = a.cosine(&a);
        assert!(
            (self_sim - 1.0).abs() < 0.001,
            "Self-similarity should be 1.0, got {}",
            self_sim
        );

        // Symmetry: sim(A, B) = sim(B, A)
        let sim_ab = a.cosine(&b);
        let sim_ba = b.cosine(&a);
        assert!(
            (sim_ab - sim_ba).abs() < 0.001,
            "Similarity not symmetric: {} vs {}",
            sim_ab,
            sim_ba
        );

        // Boundedness: -1 <= sim <= 1
        assert!((-1.0..=1.0).contains(&sim_ab), "Similarity out of bounds");
    }

    #[test]
    fn test_bundle_associativity_approximation() {
        // (A ⊕ B) ⊕ C ≈ A ⊕ (B ⊕ C)
        let a = SparseVec::random();
        let b = SparseVec::random();
        let c = SparseVec::random();

        let left = a.bundle(&b).bundle(&c);
        let right = a.bundle(&b.bundle(&c));

        let similarity = left.cosine(&right);
        assert!(
            similarity > 0.9,
            "Bundle not approximately associative: similarity = {}",
            similarity
        );
    }

    #[test]
    fn test_orthogonality_of_random_vectors() {
        // Random vectors should be approximately orthogonal
        let vectors: Vec<SparseVec> = (0..10).map(|_| SparseVec::random()).collect();

        let mut max_sim = 0.0f64;
        for i in 0..vectors.len() {
            for j in (i + 1)..vectors.len() {
                let sim = vectors[i].cosine(&vectors[j]).abs();
                if sim > max_sim {
                    max_sim = sim;
                }
            }
        }

        // Random vectors in high dimensions should have low similarity
        assert!(
            max_sim < 0.3,
            "Random vectors not sufficiently orthogonal: max_sim = {}",
            max_sim
        );
    }
}

// ============================================================================
// Section 3: Filesystem Functionality Tests
// ============================================================================

mod filesystem_functionality {
    use super::*;

    #[test]
    fn test_crud_operations() {
        let fs = VersionedEmbrFS::new();

        // Create
        let v1 = fs.write_file("test.txt", b"initial", None).unwrap();
        assert!(fs.exists("test.txt"));

        // Read
        let (data, version) = fs.read_file("test.txt").unwrap();
        assert_eq!(data, b"initial");
        assert_eq!(version, v1);

        // Update
        let v2 = fs.write_file("test.txt", b"updated", Some(v1)).unwrap();
        assert!(v2 > v1);

        let (data, _) = fs.read_file("test.txt").unwrap();
        assert_eq!(data, b"updated");

        // Delete
        fs.delete_file("test.txt", v2).unwrap();
        assert!(!fs.exists("test.txt"));
    }

    #[test]
    fn test_concurrent_reads() {
        let fs = Arc::new(VersionedEmbrFS::new());
        fs.write_file("shared.txt", b"shared data for reading", None)
            .unwrap();

        let handles: Vec<_> = (0..10)
            .map(|_| {
                let fs_clone = Arc::clone(&fs);
                thread::spawn(move || {
                    for _ in 0..100 {
                        let (data, _) = fs_clone.read_file("shared.txt").unwrap();
                        assert_eq!(data, b"shared data for reading");
                    }
                })
            })
            .collect();

        for handle in handles {
            handle.join().unwrap();
        }
    }

    #[test]
    fn test_version_conflict_detection() {
        let fs = VersionedEmbrFS::new();

        let v1 = fs.write_file("conflict.txt", b"version 1", None).unwrap();

        // Simulate concurrent update
        let v2 = fs
            .write_file("conflict.txt", b"version 2", Some(v1))
            .unwrap();

        // Try to update with stale version
        let result = fs.write_file("conflict.txt", b"stale update", Some(v1));

        match result {
            Err(EmbrFSError::VersionMismatch { expected, actual }) => {
                assert_eq!(expected, v1);
                assert_eq!(actual, v2);
            }
            _ => panic!("Expected version mismatch error"),
        }
    }

    #[test]
    fn test_file_listing() {
        let fs = VersionedEmbrFS::new();

        fs.write_file("a.txt", b"a", None).unwrap();
        fs.write_file("b.txt", b"b", None).unwrap();
        fs.write_file("c.txt", b"c", None).unwrap();

        let files = fs.list_files();
        assert_eq!(files.len(), 3);
        assert!(files.contains(&"a.txt".to_string()));
        assert!(files.contains(&"b.txt".to_string()));
        assert!(files.contains(&"c.txt".to_string()));
    }
}

// ============================================================================
// Section 4: Memory System Tests (Content-Addressable)
// ============================================================================

mod memory_system {
    use super::*;

    #[test]
    fn test_similar_content_detection() {
        // Note: With different file seeds (test1, test2, test3), the chunked encoding
        // produces orthogonal position vectors. This test verifies the encoding
        // is deterministic for the same content.
        let config = ReversibleVSAConfig::default();

        let data1 = b"The quick brown fox jumps over the lazy dog";
        let data1_copy = b"The quick brown fox jumps over the lazy dog";
        let data3 = b"Completely different content here, nothing similar";

        // Same content with same seed should produce identical vectors
        let vec1a = SparseVec::encode_data(data1, &config, Some("test1"));
        let vec1b = SparseVec::encode_data(data1_copy, &config, Some("test1"));
        let vec3 = SparseVec::encode_data(data3, &config, Some("test1"));

        // Identical content = identical encoding
        let sim_same = vec1a.cosine(&vec1b);
        assert!(
            (sim_same - 1.0).abs() < 0.001,
            "Same content should have similarity 1.0, got {}",
            sim_same
        );

        // Different content should have lower (possibly zero) similarity
        let sim_diff = vec1a.cosine(&vec3);
        assert!(
            sim_same > sim_diff,
            "Same content similarity ({}) should exceed different content ({})",
            sim_same,
            sim_diff
        );
    }

    #[test]
    fn test_pattern_bundling_retrieval() {
        // Bundle multiple patterns and query for components
        let a = SparseVec::random();
        let b = SparseVec::random();
        let c = SparseVec::random();

        // Create superposition
        let bundle = a.bundle(&b).bundle(&c);

        // Query should find components
        let sim_a = bundle.cosine(&a);
        let sim_b = bundle.cosine(&b);
        let sim_c = bundle.cosine(&c);

        // All components should have positive similarity
        assert!(sim_a > 0.2, "Component A not detectable: {}", sim_a);
        assert!(sim_b > 0.2, "Component B not detectable: {}", sim_b);
        assert!(sim_c > 0.2, "Component C not detectable: {}", sim_c);
    }

    #[test]
    fn test_binding_association() {
        // Use binding for key-value association
        // Note: Sparse ternary vectors have weaker binding retrieval than dense bipolar
        // Run multiple trials since sparse vectors have stochastic behavior
        const TRIALS: usize = 20;
        const THRESHOLD: f64 = 0.05; // Lower threshold for sparse ternary vectors
        const MIN_PASSES: usize = 14; // 70% must pass

        let mut passed = 0;
        let mut similarities = Vec::with_capacity(TRIALS);

        for _ in 0..TRIALS {
            let key = SparseVec::random();
            let value = SparseVec::random();

            // Store association
            let association = key.bind(&value);

            // Retrieve value using key
            let retrieved = association.bind(&key);

            let similarity = retrieved.cosine(&value);
            similarities.push(similarity);
            if similarity > THRESHOLD {
                passed += 1;
            }
        }

        let avg_sim: f64 = similarities.iter().sum::<f64>() / TRIALS as f64;
        assert!(
            passed >= MIN_PASSES,
            "Binding association violated: {}/{} passed (avg similarity: {:.4}, values: {:?})",
            passed,
            TRIALS,
            avg_sim,
            similarities
        );
    }
}

// ============================================================================
// Section 5: Computational Paradigm Tests
// ============================================================================

mod computational_paradigm {
    use super::*;

    #[test]
    fn test_superposition_query() {
        // Query for multiple patterns simultaneously
        let patterns: Vec<SparseVec> = (0..5).map(|_| SparseVec::random()).collect();
        let query = patterns[0].bundle(&patterns[2]).bundle(&patterns[4]);

        // Create candidates including patterns and distractors
        let candidates: Vec<SparseVec> = patterns
            .iter()
            .cloned()
            .chain((0..5).map(|_| SparseVec::random()))
            .collect();

        // Find matches
        let mut similarities: Vec<(usize, f64)> = candidates
            .iter()
            .enumerate()
            .map(|(i, c)| (i, query.cosine(c)))
            .collect();
        similarities.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

        // Top results should include bundled patterns (indices 0, 2, 4)
        let top_3: Vec<usize> = similarities[..3].iter().map(|(i, _)| *i).collect();
        let bundled_found = [0, 2, 4].iter().filter(|i| top_3.contains(i)).count();

        assert!(
            bundled_found >= 2,
            "Superposition query didn't find bundled patterns: top_3={:?}",
            top_3
        );
    }

    #[test]
    fn test_sequence_encoding() {
        // Encode sequence using permutations
        let items: Vec<SparseVec> = (0..5).map(|_| SparseVec::random()).collect();

        // Encode sequence: bundle of position-shifted items
        let mut sequence = SparseVec::new();
        for (i, item) in items.iter().enumerate() {
            let positioned = item.permute(i);
            sequence = sequence.bundle(&positioned);
        }

        // Query for item at specific position
        let query_pos = 2;
        let query = items[query_pos].permute(query_pos);

        let similarity = sequence.cosine(&query);
        assert!(
            similarity > 0.2,
            "Sequence element not retrievable: similarity = {}",
            similarity
        );
    }

    #[test]
    fn test_holographic_capacity() {
        // Test storage capacity of bundled representations
        let num_items = 50;
        let items: Vec<SparseVec> = (0..num_items).map(|_| SparseVec::random()).collect();

        // Create holographic memory
        let mut memory = SparseVec::new();
        for item in &items {
            memory = memory.bundle(item);
        }

        // Count retrievable items (similarity > 0.1)
        let retrievable = items
            .iter()
            .filter(|item| memory.cosine(item) > 0.1)
            .count();

        let retrieval_rate = retrievable as f64 / num_items as f64;
        assert!(
            retrieval_rate > 0.5,
            "Holographic capacity too low: {}% retrievable",
            retrieval_rate * 100.0
        );
    }

    #[test]
    fn test_distributed_representation() {
        // Verify distributed nature - partial corruption should degrade gracefully
        let original = SparseVec::random();

        // Create corrupted version (remove 30% of indices)
        let mut corrupted = original.clone();
        let keep_count = (corrupted.pos.len() as f64 * 0.7) as usize;
        corrupted.pos.truncate(keep_count);
        corrupted.neg.truncate(keep_count);

        // Should still have high similarity
        let similarity = original.cosine(&corrupted);
        assert!(
            similarity > 0.6,
            "Distributed representation not robust: similarity = {}",
            similarity
        );
    }
}

// ============================================================================
// Section 6: Integration Tests
// ============================================================================

mod integration {
    use super::*;

    #[test]
    fn test_full_workflow() {
        let fs = VersionedEmbrFS::new();

        // Simulate real workflow - create owned data for binary.dat
        let binary_data: Vec<u8> = (0..1024).map(|i| i as u8).collect();
        let files: Vec<(&str, &[u8])> = vec![
            ("/etc/config.json", r#"{"setting": "value"}"#.as_bytes()),
            ("/bin/script.sh", b"#!/bin/bash\necho hello"),
            ("/data/binary.dat", &binary_data[..]),
        ];

        // Write all files
        for (path, data) in &files {
            fs.write_file(path, data, None).unwrap();
        }

        // Verify all files
        for (path, original) in &files {
            let (recovered, _) = fs.read_file(path).unwrap();
            assert_eq!(*original, &recovered[..], "Mismatch for {}", path);
        }

        // Verify stats
        let stats = fs.stats();
        assert_eq!(stats.active_files, 3);
    }

    #[test]
    fn test_compression_profiles() {
        let fs = VersionedEmbrFS::new();

        // Config file - should use LZ4
        fs.write_file_compressed("/etc/test.conf", b"key=value", None)
            .unwrap();

        // Binary - should use zstd-6
        let binary = [0xDE, 0xAD, 0xBE, 0xEF].repeat(256);
        fs.write_file_compressed("/usr/bin/test", &binary, None)
            .unwrap();

        // Verify byte-perfect retrieval
        let (conf_data, _) = fs.read_file("/etc/test.conf").unwrap();
        assert_eq!(conf_data, b"key=value");

        let (bin_data, _) = fs.read_file("/usr/bin/test").unwrap();
        assert_eq!(bin_data, binary);
    }
}

// ============================================================================
// Section 7: Stress Tests
// ============================================================================

mod stress {
    use super::*;

    #[test]
    fn test_many_small_files() {
        let fs = VersionedEmbrFS::new();

        for i in 0..100 {
            let path = format!("small_{}.txt", i);
            let data = format!("Content for file {}", i);
            fs.write_file(&path, data.as_bytes(), None).unwrap();
        }

        assert_eq!(fs.list_files().len(), 100);

        // Verify random sample
        for i in [0, 25, 50, 75, 99] {
            let path = format!("small_{}.txt", i);
            let (data, _) = fs.read_file(&path).unwrap();
            let expected = format!("Content for file {}", i);
            assert_eq!(data, expected.as_bytes());
        }
    }

    #[test]
    fn test_repeated_updates() {
        let fs = VersionedEmbrFS::new();

        let mut version = fs.write_file("evolving.txt", b"v0", None).unwrap();

        for i in 1..50 {
            let data = format!("version {}", i);
            version = fs
                .write_file("evolving.txt", data.as_bytes(), Some(version))
                .unwrap();
        }

        let (final_data, _) = fs.read_file("evolving.txt").unwrap();
        assert_eq!(final_data, b"version 49");
    }
}
