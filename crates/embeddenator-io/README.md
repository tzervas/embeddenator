# embeddenator-io

<!-- FLEET-BADGES:BEGIN -->
[![CI](https://github.com/tzervas/embeddenator-io/actions/workflows/fleet-ci.yml/badge.svg?branch=main)](https://github.com/tzervas/embeddenator-io/actions/workflows/fleet-ci.yml?query=branch%3Amain)
[![Security](https://github.com/tzervas/embeddenator-io/actions/workflows/fleet-security.yml/badge.svg?branch=main)](https://github.com/tzervas/embeddenator-io/actions/workflows/fleet-security.yml?query=branch%3Amain)
<!-- FLEET-BADGES:END -->

Comprehensive I/O library for the Embeddenator ecosystem. Provides serialization, buffering, streaming, and compression utilities for efficient data handling.

**Independent component** extracted from the Embeddenator monolithic repository. Part of the [Embeddenator workspace](https://github.com/tzervas/embeddenator).

**Repository:** [https://github.com/tzervas/embeddenator-io](https://github.com/tzervas/embeddenator-io)

## Features

- **Serialization**: Binary (bincode) and JSON formats
- **Buffering**: Configurable buffer sizes for optimal performance
- **Streaming**: Memory-efficient processing of large datasets
- **Compression**: Zstandard and LZ4 support (optional)
- **Async Support**: Tokio-based async I/O (optional)
- **Envelope Format**: Binary container with compression metadata

## Status

**Production Ready** - Fully tested and documented I/O component.

## Usage

```toml
[dependencies]
embeddenator-io = { path = "../embeddenator-io" }

# Enable async support
embeddenator-io = { path = "../embeddenator-io", features = ["async"] }

# Enable compression
embeddenator-io = { path = "../embeddenator-io", features = ["compression-zstd", "compression-lz4"] }
```

### Quick Examples

#### Serialization
```rust
use embeddenator_io::*;

// Write data to file in bincode format
let data = vec![1, 2, 3, 4, 5];
write_bincode_file("data.bin", &data)?;

// Read data from file
let loaded: Vec<i32> = read_bincode_file("data.bin")?;

// JSON format
write_json_file("data.json", &data)?;
let loaded_json: Vec<i32> = read_json_file("data.json")?;
```

#### Buffering
```rust
use embeddenator_io::*;
use std::fs::File;

// Buffered file reading
let file = File::open("large_file.dat")?;
let mut reader = buffered_reader(file);

// Read in chunks
let chunks = read_chunks(&mut reader, 4096)?;

// Efficient copy
let mut src = File::open("source.dat")?;
let mut dst = File::create("dest.dat")?;
copy_buffered(&mut src, &mut dst, 65536)?;
```

#### Streaming
```rust
use embeddenator_io::*;
use std::fs::File;

// Stream large file
let file = File::open("large_data.bin")?;
let mut stream = StreamReader::new(file);

// Count bytes without loading all data
let total = stream.count_bytes()?;

// Fold operation for aggregation
let sum = stream.fold(0u64, |acc, chunk| {
    acc + chunk.len() as u64
})?;

// Stream writer
let mut output = Vec::new();
let mut writer = StreamWriter::new(&mut output);
writer.write_chunk(b"chunk1")?;
writer.write_chunk(b"chunk2")?;
writer.flush()?;
```

#### Envelope Format
```rust
use embeddenator_io::*;

// Wrap data with compression
let data = b"Some data to compress";
let wrapped = wrap_or_legacy(
    PayloadKind::EngramBincode,
    data,
    CompressionCodec::Zstd
)?;

// Unwrap automatically detects format
let unwrapped = unwrap_auto(PayloadKind::EngramBincode, &wrapped)?;
assert_eq!(unwrapped, data);
```

## Development

```bash
# Build
cargo build

# Run all tests
cargo test

# Run with all features
cargo test --all-features

# Build documentation
cargo doc --open
```

## Testing

- **Unit Tests**: 12 tests covering core functionality
- **Integration Tests**: 16 comprehensive end-to-end tests
- **Documentation Tests**: 18 examples in documentation
- **Total Coverage**: 46 tests (100% passing)

## Performance

### Buffer Sizes
- `SMALL_BUFFER_SIZE` (4KB): Small files, memory-constrained
- `DEFAULT_BUFFER_SIZE` (64KB): Optimal for most use cases
- `LARGE_BUFFER_SIZE` (1MB): Large file operations

### Format Comparison
| Format  | Size | Speed | Precision | Human Readable |
|---------|------|-------|-----------|----------------|
| Bincode | Small | Fast | Exact | No |
| JSON | Large | Slower | Approximate | Yes |

## Architecture

See [ADR-016](https://github.com/tzervas/embeddenator/blob/main/docs/adr/ADR-016-component-decomposition.md) for component decomposition rationale.

See [MIGRATION_SUMMARY.md](MIGRATION_SUMMARY.md) for detailed migration information.

## License

MIT

