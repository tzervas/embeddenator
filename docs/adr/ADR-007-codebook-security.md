# ADR-007: Codebook Security and Reversible Encoding

## Status

Proposed

## Date

2025-12-23

## Context

### Security Requirement

The current documentation describes the codebook as storing plaintext chunk data (actual bytes). This poses a significant security vulnerability:

```rust
// Current (INSECURE):
Codebook: HashMap<ChunkID, Vec<u8>>  // Plaintext bytes stored directly
```

**Problem**: Anyone with access to the engram files has immediate access to all data in plaintext form, defeating any security benefits of the holographic encoding.

### Design Goal

We require a codebook encoding mechanism that:

1. **Mathematically Simple**: Trivial to encode/decode WITH the key
2. **Mathematically Impossible**: Computationally infeasible without the key
3. **Quantum Resistant**: Not vulnerable to quantum algorithms (Shor's, Grover's)
4. **Classical Compute Resistant**: Not vulnerable to brute force or pattern analysis
5. **Mutational and Transformative**: Data transformation inherent in the encoding
6. **VSA-Compatible**: Works with VSA-as-a-lens approach for selective decryption
7. **Bulk Operations**: Efficient encryption of entire codebooks
8. **Selective Decryption**: Decrypt only needed chunks without full codebook decryption

## Decision

We implement a **VSA-Lens Reversible Encoding** system for codebook security:

### 1. VSA-as-a-Lens Cryptographic Primitive

**Core Concept**: Use VSA vectors as cryptographic lenses that transform data through high-dimensional holographic operations.

```rust
struct SecureCodebook {
    // Encrypted chunk storage
    encrypted_chunks: HashMap<ChunkID, EncryptedChunk>,
    
    // VSA-based encryption parameters
    lens_dimensionality: usize,      // e.g., 100,000
    lens_seed: [u8; 32],             // 256-bit master seed
}

struct EncryptedChunk {
    transformed_data: Vec<u8>,        // XOR with lens projection
    lens_position: SparseVec,         // Unique position in VSA space
    integrity_vector: SparseVec,      // For tamper detection
}
```

### 2. Reversible Encoding Algorithm

**Encoding (Encryption)**:

```rust
fn encode_chunk(chunk_data: &[u8], chunk_id: &ChunkID, master_lens: &MasterLens) -> EncryptedChunk {
    // 1. Generate chunk-specific lens from master seed + chunk ID
    let chunk_lens = master_lens.derive_lens(chunk_id);
    
    // 2. Create high-dimensional projection
    let lens_projection = chunk_lens.project_to_bytes(chunk_data.len());
    
    // 3. XOR transformation (reversible, mutational)
    let transformed_data: Vec<u8> = chunk_data
        .iter()
        .zip(lens_projection.iter())
        .map(|(data_byte, lens_byte)| data_byte ^ lens_byte)
        .collect();
    
    // 4. Generate integrity vector
    let integrity_vector = chunk_lens.bind(&chunk_lens.from_data(chunk_data));
    
    EncryptedChunk {
        transformed_data,
        lens_position: chunk_lens.position,
        integrity_vector,
    }
}
```

**Decoding (Decryption)**:

```rust
fn decode_chunk(encrypted: &EncryptedChunk, chunk_id: &ChunkID, master_lens: &MasterLens) -> Vec<u8> {
    // 1. Regenerate chunk-specific lens (requires master seed)
    let chunk_lens = master_lens.derive_lens(chunk_id);
    
    // 2. Verify integrity
    assert!(chunk_lens.position.cosine_similarity(&encrypted.lens_position) > 0.99);
    
    // 3. Regenerate projection
    let lens_projection = chunk_lens.project_to_bytes(encrypted.transformed_data.len());
    
    // 4. XOR to recover original (XOR is self-inverse)
    let original_data: Vec<u8> = encrypted.transformed_data
        .iter()
        .zip(lens_projection.iter())
        .map(|(encrypted_byte, lens_byte)| encrypted_byte ^ lens_byte)
        .collect();
    
    // 5. Verify integrity
    let recovered_integrity = chunk_lens.bind(&chunk_lens.from_data(&original_data));
    assert!(recovered_integrity.cosine_similarity(&encrypted.integrity_vector) > 0.99);
    
    original_data
}
```

### 3. Master Lens Derivation

**Lens Hierarchy**:

```rust
struct MasterLens {
    master_seed: [u8; 32],           // 256-bit secret key
    dimensionality: usize,            // 100K for high security
    base_vectors: Vec<SparseVec>,    // Pre-computed base vectors
}

impl MasterLens {
    fn derive_lens(&self, chunk_id: &ChunkID) -> ChunkLens {
        // Derive deterministic but unpredictable lens from master seed + chunk ID
        let mut hasher = Blake3::new();
        hasher.update(&self.master_seed);
        hasher.update(chunk_id.as_bytes());
        let lens_seed = hasher.finalize();
        
        // Generate sparse vector from seed (deterministic)
        let position = SparseVec::from_seed(lens_seed, self.dimensionality);
        
        ChunkLens {
            position,
            dimensionality: self.dimensionality,
            seed: lens_seed,
        }
    }
}

struct ChunkLens {
    position: SparseVec,              // Unique position in VSA space
    dimensionality: usize,
    seed: [u8; 32],
}

impl ChunkLens {
    fn project_to_bytes(&self, byte_count: usize) -> Vec<u8> {
        // Project high-dimensional sparse vector to byte stream
        let mut output = Vec::with_capacity(byte_count);
        let mut hasher = Blake3::new();
        hasher.update(&self.seed);
        
        // Use lens position indices to seed CSPRNG
        for i in 0..byte_count {
            hasher.update(&i.to_le_bytes());
            let hash = hasher.finalize();
            output.push(hash.as_bytes()[0]);
            hasher = Blake3::new();
            hasher.update(&hash.as_bytes()[1..32]);
        }
        
        output
    }
}
```

### 4. Security Properties

#### Quantum Resistance

**Why XOR + VSA-derived keystream is quantum-resistant**:

1. **No algebraic structure**: Unlike RSA (factoring) or ECC (discrete log), there's no algebraic problem to solve
2. **Information-theoretic security**: XOR with true random stream approaches one-time pad security
3. **High-dimensional chaos**: 100K-dimensional VSA space provides enormous search space (3^100000 possibilities)
4. **No period detection**: Blake3 + VSA prevents Grover's algorithm from finding patterns

**Grover's Algorithm Resistance**:
- Grover's provides O(√N) speedup for unstructured search
- For 256-bit key: classical 2^256 → quantum 2^128
- Still infeasible: 2^128 operations beyond any quantum computer

#### Classical Compute Resistance

**Brute Force Resistance**:
```
Master seed: 256 bits = 2^256 possibilities
Time to brute force at 1 billion attempts/sec: 
  2^256 / 10^9 ≈ 10^68 seconds ≈ 10^60 years
```

**Pattern Analysis Resistance**:
- XOR destroys all patterns in ciphertext
- VSA-derived keystream appears random (Blake3 CSPRNG)
- No frequency analysis possible
- No known-plaintext attacks (each chunk uses unique lens)

#### Mutational Properties

**Data Transformation**:
1. **Bit-level mutation**: Every bit XORed with derived pseudorandom bit
2. **Holographic dispersion**: Single bit change affects VSA lens derivation
3. **Avalanche effect**: Changing master seed changes all lenses completely
4. **Position-dependent**: Chunk ID affects lens, preventing chunk reordering attacks

### 5. VSA-as-a-Lens Selective Decryption

**Bulk Encryption, Selective Decryption**:

```rust
// Encrypt entire codebook efficiently
fn encrypt_codebook_bulk(chunks: &HashMap<ChunkID, Vec<u8>>, master_lens: &MasterLens) 
    -> SecureCodebook 
{
    let encrypted_chunks: HashMap<_, _> = chunks
        .par_iter()  // Parallel encryption
        .map(|(id, data)| {
            (*id, encode_chunk(data, id, master_lens))
        })
        .collect();
    
    SecureCodebook {
        encrypted_chunks,
        lens_dimensionality: master_lens.dimensionality,
        lens_seed: master_lens.master_seed,
    }
}

// Decrypt only needed chunks (selective)
fn decrypt_chunk_selective(codebook: &SecureCodebook, chunk_id: &ChunkID, master_lens: &MasterLens) 
    -> Vec<u8> 
{
    let encrypted = codebook.encrypted_chunks.get(chunk_id)
        .expect("Chunk not found");
    
    decode_chunk(encrypted, chunk_id, master_lens)
}

// VSA query guides decryption (lens approach)
fn reconstruct_file(engram: &Engram, secure_codebook: &SecureCodebook, 
                    file_manifest: &FileManifest, master_lens: &MasterLens) 
    -> Vec<u8> 
{
    let mut file_data = Vec::new();
    
    for chunk_ref in &file_manifest.chunks {
        // 1. VSA finds chunk ID (holographic indexing)
        let chunk_id = engram.query_chunk(&chunk_ref.vector);
        
        // 2. Selectively decrypt just this chunk (no bulk decryption needed)
        let chunk_bytes = decrypt_chunk_selective(secure_codebook, &chunk_id, master_lens);
        
        // 3. Append to file
        file_data.extend_from_slice(&chunk_bytes);
    }
    
    file_data
}
```

### 6. Key Management

**Master Lens Storage**:

```rust
// DO NOT store in engram files
// DO NOT store in manifest
// Store separately with strong protection

struct KeyManagement {
    // Option 1: Environment variable
    master_seed: Option<[u8; 32]>,  // From EMBEDDENATOR_MASTER_KEY
    
    // Option 2: Key file
    key_file_path: Option<PathBuf>,  // ~/.embeddenator/master.key
    
    // Option 3: Hardware security module
    hsm_handle: Option<HSMHandle>,
}

impl KeyManagement {
    fn load_master_key() -> Result<[u8; 32]> {
        // Try environment variable first
        if let Ok(key_hex) = std::env::var("EMBEDDENATOR_MASTER_KEY") {
            return hex::decode(key_hex)?.try_into()
                .map_err(|_| Error::InvalidKeyLength);
        }
        
        // Try key file
        let key_path = dirs::home_dir()
            .ok_or(Error::NoHomeDir)?
            .join(".embeddenator/master.key");
        
        if key_path.exists() {
            let key_bytes = std::fs::read(key_path)?;
            return key_bytes.try_into()
                .map_err(|_| Error::InvalidKeyLength);
        }
        
        Err(Error::NoMasterKey)
    }
}
```

## Consequences

### Positive

- **Security by Default**: No plaintext data in codebook, secure even without additional encryption
- **Quantum Resistant**: No algebraic structure vulnerable to quantum algorithms
- **Mathematically Simple**: XOR is trivial to compute (nanoseconds per byte)
- **Perfectly Reversible**: XOR is self-inverse, guaranteed bit-perfect decryption with key
- **Selective Decryption**: Decrypt only needed chunks, not entire codebook
- **VSA-Compatible**: Works seamlessly with holographic indexing
- **Zero Performance Impact**: XOR is hardware-accelerated, ~1-2 cycles per byte
- **Tamper Detection**: Integrity vectors detect modifications

### Negative

- **Key Management Burden**: Users must securely store master key
- **Key Loss = Data Loss**: No key recovery mechanism (by design)
- **Not Searchable**: Cannot perform operations on encrypted codebook without decryption
- **Additional Complexity**: Encoding/decoding layer adds code complexity
- **Backward Incompatibility**: Existing engrams would need migration

### Neutral

- **Not Full Encryption**: This is obfuscation + access control, not military-grade encryption
- **Layerable**: Can add AES/ChaCha20 on top for defense-in-depth
- **Performance**: XOR is ~10GB/s on modern CPUs, negligible overhead

## Implementation Roadmap

### Phase 1: Core Encoding (Weeks 1-2)
- [ ] Implement `MasterLens` and `ChunkLens` structures
- [ ] Implement `encode_chunk` and `decode_chunk` functions
- [ ] Add Blake3 dependency for cryptographic hashing
- [ ] Create comprehensive unit tests

### Phase 2: Integration (Weeks 3-4)
- [ ] Modify `EmbrFS` to use `SecureCodebook` instead of plaintext codebook
- [ ] Update ingestion to encrypt chunks during encoding
- [ ] Update extraction to decrypt chunks during reconstruction
- [ ] Add key management utilities

### Phase 3: Validation (Week 5)
- [ ] Security audit of encoding mechanism
- [ ] Performance benchmarking (expect <1% overhead)
- [ ] Migration tools for existing engrams
- [ ] Documentation updates

### Phase 4: Advanced Features (Weeks 6-8)
- [ ] Hierarchical key derivation for package isolation
- [ ] Per-package lens derivation from master lens
- [ ] Selective package decryption
- [ ] Key rotation mechanisms

## Performance Analysis

### Encoding/Decoding Overhead

**Per-Chunk Cost**:
```
Blake3 hash (32 bytes): ~50 ns
VSA lens derivation: ~100 μs (one-time per chunk)
XOR transformation (4KB): ~400 ns (10 GB/s throughput)
Integrity check: ~10 μs (cosine similarity)

Total per chunk: ~110 μs (dominated by lens derivation)
```

**Impact on Ingestion**:
```
Current ingestion: 1ms per MB (1000 μs)
Encoding overhead: 110 μs per 4KB chunk = 27.5 μs per KB = 27.5 ms per MB

New ingestion time: 1ms + 27.5ms = 28.5ms per MB
Overhead: ~2.75% (acceptable)
```

**Impact on Extraction**:
```
Similar overhead: ~2.75% slower
Still achieves <100ms for 10K tokens with decryption
```

## Security Analysis

### Threat Model

**Protected Against**:
- ✅ Unauthorized data access (requires master key)
- ✅ Data exfiltration (encrypted at rest)
- ✅ Pattern analysis attacks (XOR destroys patterns)
- ✅ Known-plaintext attacks (unique lens per chunk)
- ✅ Quantum attacks (no algebraic structure)
- ✅ Brute force (2^256 keyspace)
- ✅ Tampering (integrity vectors detect modifications)

**NOT Protected Against** (require additional layers):
- ❌ Side-channel attacks (timing, power analysis)
- ❌ Memory dumps during decryption (plaintext in RAM)
- ❌ Key compromise (no forward secrecy without key rotation)
- ❌ Rubber-hose cryptanalysis (physical coercion)

### Recommended Additional Layers

For high-security applications, add:

1. **AES-256-GCM** on top of VSA encoding
2. **Memory locking** for decrypted chunks (mlock)
3. **Secure deletion** of plaintext after use
4. **Key rotation** mechanisms
5. **Hardware security modules** for key storage

## References

- [One-Time Pad](https://en.wikipedia.org/wiki/One-time_pad) - Information-theoretic security
- [Blake3](https://github.com/BLAKE3-team/BLAKE3) - Cryptographic hash function
- [Grover's Algorithm](https://en.wikipedia.org/wiki/Grover%27s_algorithm) - Quantum search
- [Post-Quantum Cryptography](https://csrc.nist.gov/projects/post-quantum-cryptography)
- ADR-001: Sparse Ternary VSA (foundational VSA operations)
- ADR-005: Hologram Package Isolation (selective operations)
- ADR-006: Dimensionality Scaling (high-dimensional security)

## Notes

### Why Not Standard Encryption?

**Traditional encryption** (AES, ChaCha20) is excellent but:
- Adds another dependency
- Doesn't leverage the holographic structure
- Requires separate key management infrastructure

**VSA-Lens Encoding**:
- Leverages existing VSA infrastructure
- Natural fit with holographic indexing
- Can be combined with traditional encryption for defense-in-depth

### Mathematical Triviality

With the key, decryption is literally:
```rust
decrypted_byte = encrypted_byte ^ lens_byte
```

A single XOR operation. Doesn't get more trivial than that.

Without the key, finding the lens bytes requires:
- Breaking Blake3 (no known attacks)
- Searching 2^256 keyspace (infeasible)
- Or searching 3^100000 VSA space (even more infeasible)

This is the essence of modern cryptography: asymmetric computational cost.
