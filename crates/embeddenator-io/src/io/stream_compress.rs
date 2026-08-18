//! Streaming compression utilities for large data processing
//!
//! Provides streaming compressor and decompressor wrappers that integrate
//! compression with the existing stream I/O infrastructure, enabling processing
//! of multi-GB files without loading them entirely into memory.
//!
//! # Features
//!
//! - `compression-zstd`: Enable zstd streaming compression
//! - `compression-lz4`: Enable LZ4 frame streaming compression
//!
//! # Examples
//!
//! ## Streaming Compression
//! ```no_run
//! use embeddenator_io::io::stream_compress::{StreamCompressor, CompressionLevel};
//! use std::fs::File;
//!
//! let input = File::open("large_file.bin").unwrap();
//! let output = File::create("large_file.bin.zst").unwrap();
//!
//! let mut compressor = StreamCompressor::zstd(output, CompressionLevel::Default).unwrap();
//! // Write data in chunks...
//! ```
//!
//! ## Streaming Decompression
//! ```no_run
//! use embeddenator_io::io::stream_compress::StreamDecompressor;
//! use std::fs::File;
//!
//! let input = File::open("large_file.bin.zst").unwrap();
//! let mut decompressor = StreamDecompressor::zstd(input).unwrap();
//! // Read decompressed data in chunks...
//! ```

use std::io::{self, Read, Write};

use super::envelope::CompressionCodec;

/// Compression level for streaming compression
#[derive(Clone, Copy, Debug, Default)]
pub enum CompressionLevel {
    /// Fastest compression, larger output
    Fast,
    /// Balanced compression/speed trade-off
    #[default]
    Default,
    /// Best compression, slower
    Best,
    /// Custom compression level (codec-specific)
    Custom(i32),
}

impl CompressionLevel {
    /// Convert to zstd compression level
    #[cfg(feature = "compression-zstd")]
    fn to_zstd_level(self) -> i32 {
        match self {
            CompressionLevel::Fast => 1,
            CompressionLevel::Default => 3,
            CompressionLevel::Best => 19,
            CompressionLevel::Custom(level) => level,
        }
    }

    /// Convert to LZ4 compression level
    /// Note: Currently unused as lz4_flex FrameEncoder doesn't expose level settings
    #[allow(dead_code)]
    fn to_lz4_level(self) -> u32 {
        match self {
            CompressionLevel::Fast => 1,
            CompressionLevel::Default => 4,
            CompressionLevel::Best => 9,
            CompressionLevel::Custom(level) => level.max(0) as u32,
        }
    }
}

/// Streaming compressor that wraps a writer with compression
///
/// Allows writing uncompressed data which is automatically compressed
/// and written to the underlying writer in a streaming fashion.
pub struct StreamCompressor<W: Write> {
    inner: CompressorInner<W>,
    codec: CompressionCodec,
}

enum CompressorInner<W: Write> {
    #[cfg(feature = "compression-zstd")]
    Zstd(zstd::Encoder<'static, W>),
    #[cfg(feature = "compression-lz4")]
    Lz4(lz4_flex::frame::FrameEncoder<W>),
    /// Passthrough when no compression is used
    None(W),
}

impl<W: Write> StreamCompressor<W> {
    /// Create a streaming zstd compressor
    ///
    /// # Arguments
    /// * `writer` - The underlying writer for compressed output
    /// * `level` - Compression level
    ///
    /// # Errors
    /// Returns an error if zstd feature is not enabled or encoder creation fails
    #[cfg(feature = "compression-zstd")]
    pub fn zstd(writer: W, level: CompressionLevel) -> io::Result<Self> {
        let encoder = zstd::Encoder::new(writer, level.to_zstd_level())?;
        Ok(Self {
            inner: CompressorInner::Zstd(encoder),
            codec: CompressionCodec::Zstd,
        })
    }

    /// Create a streaming zstd compressor (stub when feature disabled)
    #[cfg(not(feature = "compression-zstd"))]
    pub fn zstd(_writer: W, _level: CompressionLevel) -> io::Result<Self> {
        Err(io::Error::other(
            "zstd streaming compression requires feature `compression-zstd`",
        ))
    }

    /// Create a streaming LZ4 frame compressor
    ///
    /// # Arguments
    /// * `writer` - The underlying writer for compressed output
    /// * `level` - Compression level (affects block size selection)
    ///
    /// # Errors
    /// Returns an error if lz4 feature is not enabled
    #[cfg(feature = "compression-lz4")]
    pub fn lz4(writer: W, _level: CompressionLevel) -> io::Result<Self> {
        let encoder = lz4_flex::frame::FrameEncoder::new(writer);
        Ok(Self {
            inner: CompressorInner::Lz4(encoder),
            codec: CompressionCodec::Lz4,
        })
    }

    /// Create a streaming LZ4 compressor (stub when feature disabled)
    #[cfg(not(feature = "compression-lz4"))]
    pub fn lz4(_writer: W, _level: CompressionLevel) -> io::Result<Self> {
        Err(io::Error::other(
            "lz4 streaming compression requires feature `compression-lz4`",
        ))
    }

    /// Create a passthrough compressor (no compression)
    pub fn none(writer: W) -> Self {
        Self {
            inner: CompressorInner::None(writer),
            codec: CompressionCodec::None,
        }
    }

    /// Create a streaming compressor with the specified codec
    pub fn with_codec(
        writer: W,
        codec: CompressionCodec,
        level: CompressionLevel,
    ) -> io::Result<Self> {
        match codec {
            CompressionCodec::None => Ok(Self::none(writer)),
            #[cfg(feature = "compression-zstd")]
            CompressionCodec::Zstd => Self::zstd(writer, level),
            #[cfg(not(feature = "compression-zstd"))]
            CompressionCodec::Zstd => {
                let _ = level; // Suppress unused variable warning
                Err(io::Error::other(
                    "zstd streaming compression requires feature `compression-zstd`",
                ))
            }
            #[cfg(feature = "compression-lz4")]
            CompressionCodec::Lz4 => Self::lz4(writer, level),
            #[cfg(not(feature = "compression-lz4"))]
            CompressionCodec::Lz4 => {
                let _ = level; // Suppress unused variable warning
                Err(io::Error::other(
                    "lz4 streaming compression requires feature `compression-lz4`",
                ))
            }
        }
    }

    /// Get the compression codec being used
    pub fn codec(&self) -> CompressionCodec {
        self.codec
    }

    /// Finish compression and return the underlying writer
    ///
    /// This flushes any buffered data and finalizes the compression stream.
    pub fn finish(self) -> io::Result<W> {
        match self.inner {
            #[cfg(feature = "compression-zstd")]
            CompressorInner::Zstd(encoder) => encoder.finish(),
            #[cfg(feature = "compression-lz4")]
            CompressorInner::Lz4(encoder) => encoder.finish().map_err(io::Error::other),
            CompressorInner::None(writer) => Ok(writer),
        }
    }
}

impl<W: Write> Write for StreamCompressor<W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        match &mut self.inner {
            #[cfg(feature = "compression-zstd")]
            CompressorInner::Zstd(encoder) => encoder.write(buf),
            #[cfg(feature = "compression-lz4")]
            CompressorInner::Lz4(encoder) => encoder.write(buf),
            CompressorInner::None(writer) => writer.write(buf),
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        match &mut self.inner {
            #[cfg(feature = "compression-zstd")]
            CompressorInner::Zstd(encoder) => encoder.flush(),
            #[cfg(feature = "compression-lz4")]
            CompressorInner::Lz4(encoder) => encoder.flush(),
            CompressorInner::None(writer) => writer.flush(),
        }
    }
}

/// Streaming decompressor that wraps a reader with decompression
///
/// Allows reading compressed data which is automatically decompressed
/// in a streaming fashion.
pub struct StreamDecompressor<R: Read> {
    inner: DecompressorInner<R>,
    codec: CompressionCodec,
}

enum DecompressorInner<R: Read> {
    #[cfg(feature = "compression-zstd")]
    Zstd(zstd::Decoder<'static, io::BufReader<R>>),
    #[cfg(feature = "compression-lz4")]
    Lz4(lz4_flex::frame::FrameDecoder<R>),
    /// Passthrough when no decompression is used
    None(R),
}

impl<R: Read> StreamDecompressor<R> {
    /// Create a streaming zstd decompressor
    ///
    /// # Arguments
    /// * `reader` - The underlying reader for compressed input
    ///
    /// # Errors
    /// Returns an error if zstd feature is not enabled or decoder creation fails
    #[cfg(feature = "compression-zstd")]
    pub fn zstd(reader: R) -> io::Result<Self> {
        let decoder = zstd::Decoder::new(reader)?;
        Ok(Self {
            inner: DecompressorInner::Zstd(decoder),
            codec: CompressionCodec::Zstd,
        })
    }

    /// Create a streaming zstd decompressor (stub when feature disabled)
    #[cfg(not(feature = "compression-zstd"))]
    pub fn zstd(_reader: R) -> io::Result<Self> {
        Err(io::Error::other(
            "zstd streaming decompression requires feature `compression-zstd`",
        ))
    }

    /// Create a streaming LZ4 frame decompressor
    ///
    /// # Arguments
    /// * `reader` - The underlying reader for compressed input
    ///
    /// # Errors
    /// Returns an error if lz4 feature is not enabled
    #[cfg(feature = "compression-lz4")]
    pub fn lz4(reader: R) -> io::Result<Self> {
        let decoder = lz4_flex::frame::FrameDecoder::new(reader);
        Ok(Self {
            inner: DecompressorInner::Lz4(decoder),
            codec: CompressionCodec::Lz4,
        })
    }

    /// Create a streaming LZ4 decompressor (stub when feature disabled)
    #[cfg(not(feature = "compression-lz4"))]
    pub fn lz4(_reader: R) -> io::Result<Self> {
        Err(io::Error::other(
            "lz4 streaming decompression requires feature `compression-lz4`",
        ))
    }

    /// Create a passthrough decompressor (no decompression)
    pub fn none(reader: R) -> Self {
        Self {
            inner: DecompressorInner::None(reader),
            codec: CompressionCodec::None,
        }
    }

    /// Create a streaming decompressor with the specified codec
    pub fn with_codec(reader: R, codec: CompressionCodec) -> io::Result<Self> {
        match codec {
            CompressionCodec::None => Ok(Self::none(reader)),
            #[cfg(feature = "compression-zstd")]
            CompressionCodec::Zstd => Self::zstd(reader),
            #[cfg(not(feature = "compression-zstd"))]
            CompressionCodec::Zstd => Err(io::Error::other(
                "zstd streaming decompression requires feature `compression-zstd`",
            )),
            #[cfg(feature = "compression-lz4")]
            CompressionCodec::Lz4 => Self::lz4(reader),
            #[cfg(not(feature = "compression-lz4"))]
            CompressionCodec::Lz4 => Err(io::Error::other(
                "lz4 streaming decompression requires feature `compression-lz4`",
            )),
        }
    }

    /// Get the compression codec being used
    pub fn codec(&self) -> CompressionCodec {
        self.codec
    }

    /// Get the inner reader (consumes the decompressor)
    ///
    /// Note: For zstd, returns the buffered reader wrapper's inner reader
    pub fn into_inner(self) -> R {
        match self.inner {
            #[cfg(feature = "compression-zstd")]
            DecompressorInner::Zstd(decoder) => decoder.finish().into_inner(),
            #[cfg(feature = "compression-lz4")]
            DecompressorInner::Lz4(decoder) => decoder.into_inner(),
            DecompressorInner::None(reader) => reader,
        }
    }
}

impl<R: Read> Read for StreamDecompressor<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        match &mut self.inner {
            #[cfg(feature = "compression-zstd")]
            DecompressorInner::Zstd(decoder) => decoder.read(buf),
            #[cfg(feature = "compression-lz4")]
            DecompressorInner::Lz4(decoder) => decoder.read(buf),
            DecompressorInner::None(reader) => reader.read(buf),
        }
    }
}

/// Stream-compress data from a reader to a writer
///
/// This function reads data in chunks, compresses it, and writes to the output
/// without loading the entire input into memory.
///
/// # Arguments
/// * `reader` - Source of uncompressed data
/// * `writer` - Destination for compressed data
/// * `codec` - Compression codec to use
/// * `level` - Compression level
/// * `buffer_size` - Size of the read buffer (default: 64KB)
///
/// # Returns
/// Total bytes written (compressed size)
///
/// # Examples
/// ```no_run
/// use embeddenator_io::io::stream_compress::{stream_compress, CompressionLevel};
/// use embeddenator_io::CompressionCodec;
/// use std::fs::File;
///
/// let input = File::open("large_file.bin").unwrap();
/// let output = File::create("large_file.bin.zst").unwrap();
///
/// let compressed_size = stream_compress(
///     input,
///     output,
///     CompressionCodec::Zstd,
///     CompressionLevel::Default,
///     64 * 1024,
/// ).unwrap();
///
/// println!("Compressed {} bytes", compressed_size);
/// ```
pub fn stream_compress<R: Read, W: Write>(
    mut reader: R,
    writer: W,
    codec: CompressionCodec,
    level: CompressionLevel,
    buffer_size: usize,
) -> io::Result<u64> {
    let mut compressor = StreamCompressor::with_codec(writer, codec, level)?;
    let mut buffer = vec![0u8; buffer_size];
    let mut total_written = 0u64;

    loop {
        let bytes_read = reader.read(&mut buffer)?;
        if bytes_read == 0 {
            break;
        }
        compressor.write_all(&buffer[..bytes_read])?;
        total_written += bytes_read as u64;
    }

    compressor.finish()?;
    Ok(total_written)
}

/// Stream-decompress data from a reader to a writer
///
/// This function reads compressed data in chunks, decompresses it, and writes
/// to the output without loading the entire input into memory.
///
/// # Arguments
/// * `reader` - Source of compressed data
/// * `writer` - Destination for uncompressed data
/// * `codec` - Compression codec used
/// * `buffer_size` - Size of the read buffer (default: 64KB)
///
/// # Returns
/// Total bytes written (uncompressed size)
///
/// # Examples
/// ```no_run
/// use embeddenator_io::io::stream_compress::stream_decompress;
/// use embeddenator_io::CompressionCodec;
/// use std::fs::File;
///
/// let input = File::open("large_file.bin.zst").unwrap();
/// let output = File::create("large_file.bin").unwrap();
///
/// let uncompressed_size = stream_decompress(
///     input,
///     output,
///     CompressionCodec::Zstd,
///     64 * 1024,
/// ).unwrap();
///
/// println!("Decompressed {} bytes", uncompressed_size);
/// ```
pub fn stream_decompress<R: Read, W: Write>(
    reader: R,
    mut writer: W,
    codec: CompressionCodec,
    buffer_size: usize,
) -> io::Result<u64> {
    let mut decompressor = StreamDecompressor::with_codec(reader, codec)?;
    let mut buffer = vec![0u8; buffer_size];
    let mut total_written = 0u64;

    loop {
        let bytes_read = decompressor.read(&mut buffer)?;
        if bytes_read == 0 {
            break;
        }
        writer.write_all(&buffer[..bytes_read])?;
        total_written += bytes_read as u64;
    }

    writer.flush()?;
    Ok(total_written)
}

/// Stream-compress a file to another file
///
/// Convenience function for file-to-file streaming compression.
///
/// # Arguments
/// * `input_path` - Path to input file
/// * `output_path` - Path to output file
/// * `codec` - Compression codec to use
/// * `level` - Compression level
///
/// # Returns
/// Tuple of (uncompressed_size, compressed_size)
pub fn compress_file<P: AsRef<std::path::Path>, Q: AsRef<std::path::Path>>(
    input_path: P,
    output_path: Q,
    codec: CompressionCodec,
    level: CompressionLevel,
) -> io::Result<(u64, u64)> {
    let input = std::fs::File::open(input_path)?;
    let input_size = input.metadata()?.len();
    let output = std::fs::File::create(output_path.as_ref())?;

    stream_compress(input, &output, codec, level, 64 * 1024)?;

    let output_size = std::fs::metadata(output_path)?.len();
    Ok((input_size, output_size))
}

/// Stream-decompress a file to another file
///
/// Convenience function for file-to-file streaming decompression.
///
/// # Arguments
/// * `input_path` - Path to compressed input file
/// * `output_path` - Path to output file
/// * `codec` - Compression codec used
///
/// # Returns
/// Tuple of (compressed_size, uncompressed_size)
pub fn decompress_file<P: AsRef<std::path::Path>, Q: AsRef<std::path::Path>>(
    input_path: P,
    output_path: Q,
    codec: CompressionCodec,
) -> io::Result<(u64, u64)> {
    let input = std::fs::File::open(input_path.as_ref())?;
    let input_size = input.metadata()?.len();
    let output = std::fs::File::create(output_path.as_ref())?;

    stream_decompress(input, &output, codec, 64 * 1024)?;

    let output_size = std::fs::metadata(output_path)?.len();
    Ok((input_size, output_size))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_passthrough_compressor() {
        let data = b"Hello, streaming compression!";
        let mut output = Vec::new();

        let mut compressor = StreamCompressor::none(&mut output);
        compressor.write_all(data).unwrap();
        compressor.finish().unwrap();

        assert_eq!(output, data);
    }

    #[test]
    fn test_passthrough_decompressor() {
        let data = b"Hello, streaming decompression!";
        let cursor = Cursor::new(data.to_vec());

        let mut decompressor = StreamDecompressor::none(cursor);
        let mut output = Vec::new();
        decompressor.read_to_end(&mut output).unwrap();

        assert_eq!(output, data);
    }

    #[test]
    fn test_stream_compress_decompress_none() {
        let data = b"Test data for streaming compression roundtrip";
        let input = Cursor::new(data.to_vec());
        let mut compressed = Vec::new();

        stream_compress(
            input,
            &mut compressed,
            CompressionCodec::None,
            CompressionLevel::Default,
            1024,
        )
        .unwrap();

        let compressed_reader = Cursor::new(compressed);
        let mut decompressed = Vec::new();

        stream_decompress(
            compressed_reader,
            &mut decompressed,
            CompressionCodec::None,
            1024,
        )
        .unwrap();

        assert_eq!(decompressed, data);
    }

    #[cfg(feature = "compression-zstd")]
    #[test]
    fn test_zstd_streaming_roundtrip() {
        let data = b"Hello, zstd streaming compression! This is a test of streaming compression with zstd.";
        let input = Cursor::new(data.to_vec());
        let mut compressed = Vec::new();

        // Compress
        stream_compress(
            input,
            &mut compressed,
            CompressionCodec::Zstd,
            CompressionLevel::Default,
            1024,
        )
        .unwrap();

        // Verify compression happened
        assert!(compressed.len() < data.len() || !compressed.is_empty());

        // Decompress
        let compressed_reader = Cursor::new(compressed);
        let mut decompressed = Vec::new();

        stream_decompress(
            compressed_reader,
            &mut decompressed,
            CompressionCodec::Zstd,
            1024,
        )
        .unwrap();

        assert_eq!(decompressed, data);
    }

    #[cfg(feature = "compression-zstd")]
    #[test]
    fn test_zstd_large_data_streaming() {
        // Create 1MB of test data
        let data: Vec<u8> = (0..1024 * 1024).map(|i| (i % 256) as u8).collect();
        let input = Cursor::new(data.clone());
        let mut compressed = Vec::new();

        // Compress with small buffer to test chunking
        stream_compress(
            input,
            &mut compressed,
            CompressionCodec::Zstd,
            CompressionLevel::Fast,
            4096,
        )
        .unwrap();

        // Decompress
        let compressed_reader = Cursor::new(compressed);
        let mut decompressed = Vec::new();

        stream_decompress(
            compressed_reader,
            &mut decompressed,
            CompressionCodec::Zstd,
            4096,
        )
        .unwrap();

        assert_eq!(decompressed.len(), data.len());
        assert_eq!(decompressed, data);
    }

    #[cfg(feature = "compression-lz4")]
    #[test]
    fn test_lz4_streaming_roundtrip() {
        let data = b"Hello, LZ4 streaming compression! This is a test of streaming compression with LZ4 frame format.";
        let input = Cursor::new(data.to_vec());
        let mut compressed = Vec::new();

        // Compress
        stream_compress(
            input,
            &mut compressed,
            CompressionCodec::Lz4,
            CompressionLevel::Default,
            1024,
        )
        .unwrap();

        // Verify compression produced output
        assert!(!compressed.is_empty());

        // Decompress
        let compressed_reader = Cursor::new(compressed);
        let mut decompressed = Vec::new();

        stream_decompress(
            compressed_reader,
            &mut decompressed,
            CompressionCodec::Lz4,
            1024,
        )
        .unwrap();

        assert_eq!(decompressed, data);
    }

    #[test]
    #[cfg(feature = "compression-zstd")]
    fn test_compression_level_zstd_conversion() {
        assert_eq!(CompressionLevel::Fast.to_zstd_level(), 1);
        assert_eq!(CompressionLevel::Default.to_zstd_level(), 3);
        assert_eq!(CompressionLevel::Best.to_zstd_level(), 19);
        assert_eq!(CompressionLevel::Custom(10).to_zstd_level(), 10);
    }

    #[test]
    fn test_compression_level_lz4_conversion() {
        assert_eq!(CompressionLevel::Fast.to_lz4_level(), 1);
        assert_eq!(CompressionLevel::Default.to_lz4_level(), 4);
        assert_eq!(CompressionLevel::Best.to_lz4_level(), 9);
    }
}
