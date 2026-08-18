# embeddenator-interop

<!-- FLEET-BADGES:BEGIN -->
[![CI](https://github.com/tzervas/embeddenator-interop/actions/workflows/fleet-ci.yml/badge.svg?branch=main)](https://github.com/tzervas/embeddenator-interop/actions/workflows/fleet-ci.yml?query=branch%3Amain)
[![Security](https://github.com/tzervas/embeddenator-interop/actions/workflows/fleet-security.yml/badge.svg?branch=main)](https://github.com/tzervas/embeddenator-interop/actions/workflows/fleet-security.yml?query=branch%3Amain)
<!-- FLEET-BADGES:END -->

**Interoperability layer for Embeddenator**: format conversions, FFI bindings, and language integrations.

**Independent component** extracted from the Embeddenator monolithic repository. Part of the [Embeddenator workspace](https://github.com/tzervas/embeddenator).

**Repository:** [https://github.com/tzervas/embeddenator-interop](https://github.com/tzervas/embeddenator-interop)

## Status

**** - Core functionality complete.

Implementation includes:
-  Format conversion (JSON, bincode, text)
-  C/C++ FFI bindings with automated header generation
-  Python bindings (PyO3) - requires `python` feature
-  Envelope compression support (Zstd, LZ4)
-  High-level adapter layers

## Features

### Format Conversions
- **JSON**: Human-readable, cross-language compatible
- **Bincode**: Efficient binary serialization
- **Text**: Debug-friendly output format
- Round-trip guarantees for JSON and bincode formats
- Support for all core Embeddenator types (SparseVec, Engram, Manifest, VSAConfig)

### FFI Bindings (C/C++)
- C-compatible interface for cross-language integration
- Opaque handle-based API for memory safety
- Core operations: encode, decode, bundle, bind, cosine similarity
- Serialization to/from JSON for data interchange
- Well-documented safety requirements

### Python Bindings (Optional)
- PyO3-based Python API (enable `python` feature)
- Pythonic interface with property accessors
- Native integration with Python bytes and strings
- JSON and bincode serialization support

### Adapter Layers
- **EnvelopeAdapter**: Full compression support with Zstd and LZ4 codecs
- **FileAdapter**: High-level file I/O operations
- **StreamAdapter**: Streaming encode/decode for large data
- **BatchAdapter**: Batch operations for efficiency
- **AutoFormatAdapter**: Automatic format detection

### Compression Support
- **Zstd**: High compression ratio (enable `compression-zstd` feature)
- **LZ4**: Fast compression (enable `compression-lz4` feature)
- **None**: No compression for maximum speed
- Full round-trip guarantees for all compression codecs

### Automated C Header Generation
- Automatic C header generation via cbindgen (enable `c-bindings` feature)
- Headers generated in `include/embeddenator_interop.h`
- C++ compatible with proper include guards
- Full documentation included in generated headers

### Kernel Interop
- Backend-agnostic VSA operations
- Vector store abstraction
- Candidate generation and reranking
- Runtime integration support

## Usage

### Format Conversion

```rust
use embeddenator_interop::formats::{sparse_vec_to_format, sparse_vec_from_format, OutputFormat};
use embeddenator_vsa::SparseVec;

// Create a vector
let vec = SparseVec {
    pos: vec![1, 2, 3],
    neg: vec![4, 5],
};

// Convert to JSON
let json_bytes = sparse_vec_to_format(&vec, OutputFormat::JsonPretty).unwrap();
let from_json = sparse_vec_from_format(&json_bytes, OutputFormat::Json).unwrap();

// Convert to bincode
let bincode_bytes = sparse_vec_to_format(&vec, OutputFormat::Bincode).unwrap();
let from_bincode = sparse_vec_from_format(&bincode_bytes, OutputFormat::Bincode).unwrap();

// Debug text format
let text = sparse_vec_to_format(&vec, OutputFormat::Text).unwrap();
println!("{}", String::from_utf8(text).unwrap());
```

### File Operations

```rust
use embeddenator_interop::FileAdapter;
use embeddenator_vsa::{SparseVec, ReversibleVSAConfig};
use embeddenator_fs::Manifest;

// Save and load sparse vectors
let vec = SparseVec::new();
FileAdapter::save_sparse_vec("vector.bin", &vec).unwrap();
let loaded = FileAdapter::load_sparse_vec("vector.bin").unwrap();

// Save and load config
let config = ReversibleVSAConfig::default();
FileAdapter::save_vsa_config("config.json", &config).unwrap();
let loaded_config = FileAdapter::load_vsa_config("config.json").unwrap();

// Save and load manifests
let manifest = Manifest {
    files: Vec::new(),
    total_chunks: 0,
};
FileAdapter::save_manifest("manifest.json", &manifest).unwrap();
let loaded_manifest = FileAdapter::load_manifest("manifest.json").unwrap();
```

### Batch Operations

```rust
use embeddenator_interop::BatchAdapter;
use embeddenator_vsa::ReversibleVSAConfig;

let config = ReversibleVSAConfig::default();
let data_chunks = vec![b"hello".as_slice(), b"world".as_slice()];

// Batch encode
let vectors = BatchAdapter::batch_encode(&data_chunks, &config);

// Batch similarity
let query = vectors[0].clone();
let similarities = BatchAdapter::batch_similarity(&query, &vectors);

// Batch bundle
let bundled = BatchAdapter::batch_bundle(&vectors);
```

### C FFI Example

```c
#include "embeddenator_interop.h"

// Create vectors
SparseVecHandle* vec1 = sparse_vec_new();
SparseVecHandle* vec2 = sparse_vec_new();

// Perform operations
SparseVecHandle* bundled = sparse_vec_bundle(vec1, vec2);
double similarity = sparse_vec_cosine(vec1, vec2);

// Encode data
VSAConfigHandle* config = vsa_config_new();
const char* data = "Hello, C!";
SparseVecHandle* encoded = vsa_encode_data(config, (const uint8_t*)data, strlen(data), NULL);

// Serialize to JSON
ByteBuffer json = sparse_vec_to_json(encoded);
// Use json.data, json.len...
byte_buffer_free(json);

// Cleanup
sparse_vec_free(vec1);
sparse_vec_free(vec2);
sparse_vec_free(bundled);
sparse_vec_free(encoded);
vsa_config_free(config);
```

### Python Example

```python
from embeddenator_interop import SparseVec, VSAConfig

# Create vectors
vec1 = SparseVec.from_indices([1, 2, 3], [4, 5])
vec2 = SparseVec.from_indices([2, 3, 4], [5, 6])

# Operations
bundled = vec1.bundle(vec2)
similarity = vec1.cosine(vec2)

# Encode data
config = VSAConfig.new()
data = b"Hello, Python!"
encoded = config.encode(data, None)

# Serialize
json_str = encoded.to_json()
bytes_data = encoded.to_bytes()
```

## Features

Default features: None

Optional features:
- `python`: Enable Python bindings via PyO3
- `c-bindings`: Enable automated C header generation with cbindgen
- `compression-zstd`: Enable Zstd compression codec
- `compression-lz4`: Enable LZ4 compression codec
- `compression`: Enable all compression codecs (zstd + lz4)

## Dependencies

```toml
[dependencies]
embeddenator-interop = { version = "0.20.0-alpha.1" }

# With Python support
embeddenator-interop = { version = "0.20.0-alpha.1", features = ["python"] }
```

## Development

### Build

```bash
# Standard build
cargo build --manifest-path embeddenator-interop/Cargo.toml

# With compression support
cargo build --manifest-path embeddenator-interop/Cargo.toml --features compression

# Generate C headers (creates include/embeddenator_interop.h)
cargo build --manifest-path embeddenator-interop/Cargo.toml --features c-bindings

# With all features
cargo build --manifest-path embeddenator-interop/Cargo.toml --all-features

# With Python support
cargo build --manifest-path embeddenator-interop/Cargo.toml --features python

# Release build
cargo build --manifest-path embeddenator-interop/Cargo.toml --release
```

### Test

```bash
# Run all tests
cargo test --manifest-path embeddenator-interop/Cargo.toml

# Run with Python tests
cargo test --manifest-path embeddenator-interop/Cargo.toml --features python
```

## Architecture

- **formats.rs**: Format conversion utilities (JSON, bincode, text)
- **ffi.rs**: C FFI bindings with opaque handles
- **bindings.rs**: Python bindings via PyO3 (optional)
- **adapters.rs**: High-level adapter layers
- **kernel_interop.rs**: Backend-agnostic VSA operations

## Supported Formats

| Type | JSON | Bincode | Text |
|------|------|---------|------|
| SparseVec | ✓ | ✓ | ✓ (read-only) |
| Engram | ✓ | ✓ | ✓ (read-only) |
| Manifest | ✓ | ✓ | ✓ (read-only) |
| SubEngram | ✓ | ✓ | ✓ (read-only) |
| VSAConfig | ✓ | ✓ | ✓ (read-only) |

## FFI Safety

All FFI functions are marked `unsafe` and require:
- Valid, non-null pointers
- Proper memory management (caller frees returned memory)
- Null-terminated UTF-8 strings
- No use-after-free violations

See FFI documentation for detailed safety contracts.

## Integration Recommendations

### For C/C++ Projects
1. Include generated header (requires cbindgen)
2. Link against libraryembeddenator_interop.a or .so
3. Follow opaque handle pattern
4. Always free allocated resources

### For Python Projects
1. Build with `--features python`
2. Install as Python module
3. Use Pythonic interface with native types
4. Serialization integrates with pickle/JSON

### For Rust Projects
1. Use adapter layers for high-level operations
2. Use formats module for conversion needs
3. Use kernel_interop for backend integration
4. Direct access to all functionality

## Performance Notes

- Bincode is ~10x faster than JSON for serialization
- Batch operations reduce overhead for multiple items
- Streaming adapters minimize memory usage for large data
- FFI calls have minimal overhead (single indirection)

## License

MIT

## See Also

- [ADR-016](https://github.com/tzervas/embeddenator/blob/main/docs/adr/ADR-016-component-decomposition.md) - Component decomposition rationale
- [embeddenator](https://github.com/tzervas/embeddenator) - Main repository
- [embeddenator-vsa](https://github.com/tzervas/embeddenator-vsa) - VSA implementation
- [embeddenator-fs](https://github.com/tzervas/embeddenator-fs) - Filesystem types
- [embeddenator-io](https://github.com/tzervas/embeddenator-io) - I/O utilities
