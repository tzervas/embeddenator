//! Buffering utilities for efficient I/O operations
//!
//! Provides buffered readers and writers with configurable buffer sizes
//! and chunking strategies optimized for different data patterns.

use std::fs::File;
use std::io::{self, BufReader, BufWriter, Read, Write};
use std::path::Path;

/// Default buffer size for I/O operations (64KB)
pub const DEFAULT_BUFFER_SIZE: usize = 64 * 1024;

/// Large buffer size for high-throughput operations (1MB)
pub const LARGE_BUFFER_SIZE: usize = 1024 * 1024;

/// Small buffer size for memory-constrained scenarios (4KB)
pub const SMALL_BUFFER_SIZE: usize = 4 * 1024;

/// Create a buffered reader with default buffer size
///
/// # Examples
/// ```no_run
/// use embeddenator_io::buffered_reader;
/// use std::io::Read;
///
/// let mut reader = buffered_reader("data.bin").unwrap();
/// let mut buffer = Vec::new();
/// reader.read_to_end(&mut buffer).unwrap();
/// ```
pub fn buffered_reader<P: AsRef<Path>>(path: P) -> io::Result<BufReader<File>> {
    let file = File::open(path)?;
    Ok(BufReader::with_capacity(DEFAULT_BUFFER_SIZE, file))
}

/// Create a buffered reader with custom buffer size
pub fn buffered_reader_with_capacity<P: AsRef<Path>>(
    path: P,
    capacity: usize,
) -> io::Result<BufReader<File>> {
    let file = File::open(path)?;
    Ok(BufReader::with_capacity(capacity, file))
}

/// Create a buffered writer with default buffer size
///
/// # Examples
/// ```no_run
/// use embeddenator_io::buffered_writer;
/// use std::io::Write;
///
/// let mut writer = buffered_writer("output.bin").unwrap();
/// writer.write_all(b"Hello, world!").unwrap();
/// writer.flush().unwrap();
/// ```
pub fn buffered_writer<P: AsRef<Path>>(path: P) -> io::Result<BufWriter<File>> {
    let file = File::create(path)?;
    Ok(BufWriter::with_capacity(DEFAULT_BUFFER_SIZE, file))
}

/// Create a buffered writer with custom buffer size
pub fn buffered_writer_with_capacity<P: AsRef<Path>>(
    path: P,
    capacity: usize,
) -> io::Result<BufWriter<File>> {
    let file = File::create(path)?;
    Ok(BufWriter::with_capacity(capacity, file))
}

/// Read a file in chunks, applying a function to each chunk
///
/// This is useful for processing large files without loading them entirely into memory.
///
/// # Examples
/// ```no_run
/// use embeddenator_io::read_chunks;
///
/// let mut total_size = 0;
/// read_chunks("large_file.bin", 4096, |chunk| {
///     total_size += chunk.len();
///     Ok(())
/// }).unwrap();
/// println!("Total size: {} bytes", total_size);
/// ```
pub fn read_chunks<P, F>(path: P, chunk_size: usize, mut callback: F) -> io::Result<()>
where
    P: AsRef<Path>,
    F: FnMut(&[u8]) -> io::Result<()>,
{
    let file = File::open(path)?;
    let mut reader = BufReader::with_capacity(chunk_size.max(4096), file);
    let mut buffer = vec![0u8; chunk_size];

    loop {
        let n = reader.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        callback(&buffer[..n])?;
    }

    Ok(())
}

/// Write data to a file in chunks
///
/// # Examples
/// ```no_run
/// use embeddenator_io::write_chunks;
///
/// let data = vec![b"chunk1", b"chunk2", b"chunk3"];
/// write_chunks("output.bin", &data).unwrap();
/// ```
pub fn write_chunks<P, I, D>(path: P, chunks: I) -> io::Result<()>
where
    P: AsRef<Path>,
    I: IntoIterator<Item = D>,
    D: AsRef<[u8]>,
{
    let file = File::create(path)?;
    let mut writer = BufWriter::with_capacity(DEFAULT_BUFFER_SIZE, file);

    for chunk in chunks {
        writer.write_all(chunk.as_ref())?;
    }

    writer.flush()?;
    Ok(())
}

/// Copy data from reader to writer with buffering
///
/// Returns the number of bytes copied.
///
/// # Examples
/// ```no_run
/// use embeddenator_io::copy_buffered;
/// use std::fs::File;
///
/// let mut src = File::open("input.bin").unwrap();
/// let mut dst = File::create("output.bin").unwrap();
/// let bytes_copied = copy_buffered(&mut src, &mut dst, 64 * 1024).unwrap();
/// println!("Copied {} bytes", bytes_copied);
/// ```
pub fn copy_buffered<R: Read, W: Write>(
    reader: &mut R,
    writer: &mut W,
    buffer_size: usize,
) -> io::Result<u64> {
    let mut buffer = vec![0u8; buffer_size];
    let mut total = 0u64;

    loop {
        let n = reader.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        writer.write_all(&buffer[..n])?;
        total += n as u64;
    }

    Ok(total)
}

/// Stream processor for chunked data processing
pub struct ChunkStream<R> {
    reader: BufReader<R>,
    chunk_size: usize,
}

impl<R: Read> ChunkStream<R> {
    /// Create a new chunk stream with default chunk size
    pub fn new(reader: R) -> Self {
        Self::with_chunk_size(reader, DEFAULT_BUFFER_SIZE)
    }

    /// Create a new chunk stream with custom chunk size
    pub fn with_chunk_size(reader: R, chunk_size: usize) -> Self {
        Self {
            reader: BufReader::with_capacity(chunk_size.max(4096), reader),
            chunk_size,
        }
    }

    /// Read the next chunk
    ///
    /// Returns `None` when the end of the stream is reached.
    pub fn next_chunk(&mut self) -> io::Result<Option<Vec<u8>>> {
        let mut buffer = vec![0u8; self.chunk_size];
        let n = self.reader.read(&mut buffer)?;
        if n == 0 {
            return Ok(None);
        }
        buffer.truncate(n);
        Ok(Some(buffer))
    }

    /// Process all chunks with a callback
    pub fn process_all<F>(&mut self, mut callback: F) -> io::Result<()>
    where
        F: FnMut(&[u8]) -> io::Result<()>,
    {
        loop {
            let mut buffer = vec![0u8; self.chunk_size];
            let n = self.reader.read(&mut buffer)?;
            if n == 0 {
                break;
            }
            callback(&buffer[..n])?;
        }
        Ok(())
    }
}

#[cfg(feature = "async")]
pub mod async_buffer {
    //! Async variants of buffering functions

    use std::io;
    use std::path::Path;
    use tokio::fs::File;
    use tokio::io::{AsyncReadExt, AsyncWriteExt, BufReader, BufWriter};

    use super::DEFAULT_BUFFER_SIZE;

    /// Create an async buffered reader
    pub async fn buffered_reader<P: AsRef<Path>>(path: P) -> io::Result<BufReader<File>> {
        let file = File::open(path).await?;
        Ok(BufReader::with_capacity(DEFAULT_BUFFER_SIZE, file))
    }

    /// Create an async buffered writer
    pub async fn buffered_writer<P: AsRef<Path>>(path: P) -> io::Result<BufWriter<File>> {
        let file = File::create(path).await?;
        Ok(BufWriter::with_capacity(DEFAULT_BUFFER_SIZE, file))
    }

    /// Read a file in chunks asynchronously
    pub async fn read_chunks<P, F, Fut>(
        path: P,
        chunk_size: usize,
        mut callback: F,
    ) -> io::Result<()>
    where
        P: AsRef<Path>,
        F: FnMut(Vec<u8>) -> Fut,
        Fut: std::future::Future<Output = io::Result<()>>,
    {
        let file = File::open(path).await?;
        let mut reader = BufReader::with_capacity(chunk_size.max(4096), file);
        let mut buffer = vec![0u8; chunk_size];

        loop {
            let n = reader.read(&mut buffer).await?;
            if n == 0 {
                break;
            }
            callback(buffer[..n].to_vec()).await?;
        }

        Ok(())
    }

    /// Copy data asynchronously with buffering
    pub async fn copy_buffered<R, W>(
        reader: &mut R,
        writer: &mut W,
        buffer_size: usize,
    ) -> io::Result<u64>
    where
        R: AsyncReadExt + Unpin,
        W: AsyncWriteExt + Unpin,
    {
        let mut buffer = vec![0u8; buffer_size];
        let mut total = 0u64;

        loop {
            let n = reader.read(&mut buffer).await?;
            if n == 0 {
                break;
            }
            writer.write_all(&buffer[..n]).await?;
            total += n as u64;
        }

        Ok(total)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_chunk_stream() {
        let data = b"Hello, world! This is a test.";
        let cursor = Cursor::new(data);
        let mut stream = ChunkStream::with_chunk_size(cursor, 10);

        let mut chunks = Vec::new();
        while let Some(chunk) = stream.next_chunk().unwrap() {
            chunks.push(chunk);
        }

        let reconstructed: Vec<u8> = chunks.into_iter().flatten().collect();
        assert_eq!(reconstructed, data);
    }

    #[test]
    fn test_copy_buffered() {
        let data = b"Test data for copying";
        let mut reader = Cursor::new(data);
        let mut writer = Vec::new();

        let copied = copy_buffered(&mut reader, &mut writer, 8).unwrap();
        assert_eq!(copied, data.len() as u64);
        assert_eq!(writer, data);
    }

    #[test]
    fn test_process_all() {
        let data = b"Process all chunks";
        let cursor = Cursor::new(data);
        let mut stream = ChunkStream::with_chunk_size(cursor, 5);

        let mut total_bytes = 0;
        stream
            .process_all(|chunk| {
                total_bytes += chunk.len();
                Ok(())
            })
            .unwrap();

        assert_eq!(total_bytes, data.len());
    }
}
