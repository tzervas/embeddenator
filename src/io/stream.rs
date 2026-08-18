//! Streaming I/O utilities for processing large datasets
//!
//! Provides efficient streaming interfaces for reading and writing data
//! without loading entire files into memory.

use std::io::{self, Read, Write};
use std::path::Path;

/// Stream reader for processing data in chunks
pub struct StreamReader<R> {
    reader: R,
    buffer_size: usize,
}

impl<R: Read> StreamReader<R> {
    /// Create a new stream reader with default buffer size
    pub fn new(reader: R) -> Self {
        Self::with_buffer_size(reader, super::buffer::DEFAULT_BUFFER_SIZE)
    }

    /// Create a new stream reader with custom buffer size
    pub fn with_buffer_size(reader: R, buffer_size: usize) -> Self {
        Self {
            reader,
            buffer_size,
        }
    }

    /// Read all data and apply a transformation function
    pub fn read_all<F, T>(&mut self, mut transform: F) -> io::Result<Vec<T>>
    where
        F: FnMut(&[u8]) -> io::Result<T>,
    {
        let mut results = Vec::new();
        let mut buffer = vec![0u8; self.buffer_size];

        loop {
            let n = self.reader.read(&mut buffer)?;
            if n == 0 {
                break;
            }
            let result = transform(&buffer[..n])?;
            results.push(result);
        }

        Ok(results)
    }

    /// Read data and fold it into an accumulator
    pub fn fold<F, T>(&mut self, init: T, mut fold_fn: F) -> io::Result<T>
    where
        F: FnMut(T, &[u8]) -> io::Result<T>,
    {
        let mut acc = init;
        let mut buffer = vec![0u8; self.buffer_size];

        loop {
            let n = self.reader.read(&mut buffer)?;
            if n == 0 {
                break;
            }
            acc = fold_fn(acc, &buffer[..n])?;
        }

        Ok(acc)
    }

    /// Count bytes in the stream
    pub fn count_bytes(&mut self) -> io::Result<u64> {
        self.fold(0u64, |acc, chunk| Ok(acc + chunk.len() as u64))
    }
}

/// Stream writer for efficient data output
pub struct StreamWriter<W> {
    writer: W,
    buffer: Vec<u8>,
    buffer_size: usize,
}

impl<W: Write> StreamWriter<W> {
    /// Create a new stream writer with default buffer size
    pub fn new(writer: W) -> Self {
        Self::with_buffer_size(writer, super::buffer::DEFAULT_BUFFER_SIZE)
    }

    /// Create a new stream writer with custom buffer size
    pub fn with_buffer_size(writer: W, buffer_size: usize) -> Self {
        Self {
            writer,
            buffer: Vec::with_capacity(buffer_size),
            buffer_size,
        }
    }

    /// Write a chunk of data
    pub fn write_chunk(&mut self, data: &[u8]) -> io::Result<()> {
        // If data fits in buffer, append it
        if self.buffer.len() + data.len() <= self.buffer_size {
            self.buffer.extend_from_slice(data);
            return Ok(());
        }

        // Flush current buffer
        if !self.buffer.is_empty() {
            self.writer.write_all(&self.buffer)?;
            self.buffer.clear();
        }

        // If data is larger than buffer, write directly
        if data.len() > self.buffer_size {
            self.writer.write_all(data)?;
        } else {
            self.buffer.extend_from_slice(data);
        }

        Ok(())
    }

    /// Flush any buffered data
    pub fn flush(&mut self) -> io::Result<()> {
        if !self.buffer.is_empty() {
            self.writer.write_all(&self.buffer)?;
            self.buffer.clear();
        }
        self.writer.flush()
    }

    /// Finish writing and return the inner writer
    pub fn finish(mut self) -> io::Result<W> {
        self.flush()?;
        Ok(self.writer)
    }
}

/// Write data to a file in streaming fashion
///
/// # Examples
/// ```no_run
/// use embeddenator_io::stream_write_file;
///
/// let chunks: Vec<&[u8]> = vec![b"Hello, ", b"world!"];
/// stream_write_file("output.txt", chunks.iter().copied()).unwrap();
/// ```
pub fn stream_write_file<P, I, D>(path: P, chunks: I) -> io::Result<()>
where
    P: AsRef<Path>,
    I: Iterator<Item = D>,
    D: AsRef<[u8]>,
{
    let file = std::fs::File::create(path)?;
    let mut writer = StreamWriter::new(file);

    for chunk in chunks {
        writer.write_chunk(chunk.as_ref())?;
    }

    writer.flush()?;
    Ok(())
}

/// Read a file in streaming fashion
///
/// # Examples
/// ```no_run
/// use embeddenator_io::stream_read_file;
///
/// let mut total_size = 0;
/// stream_read_file("input.txt", |chunk| {
///     total_size += chunk.len();
///     Ok(())
/// }).unwrap();
/// println!("Total size: {} bytes", total_size);
/// ```
pub fn stream_read_file<P, F>(path: P, mut callback: F) -> io::Result<()>
where
    P: AsRef<Path>,
    F: FnMut(&[u8]) -> io::Result<()>,
{
    let file = std::fs::File::open(path)?;
    let mut reader = StreamReader::new(file);
    let mut buffer = vec![0u8; reader.buffer_size];

    loop {
        let n = reader.reader.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        callback(&buffer[..n])?;
    }

    Ok(())
}

#[cfg(feature = "async")]
pub mod async_stream {
    //! Async streaming I/O utilities

    use std::io;
    use std::path::Path;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    use super::super::buffer::DEFAULT_BUFFER_SIZE;

    /// Async stream reader
    pub struct AsyncStreamReader<R> {
        reader: R,
        buffer_size: usize,
    }

    impl<R: AsyncReadExt + Unpin> AsyncStreamReader<R> {
        /// Create a new async stream reader
        pub fn new(reader: R) -> Self {
            Self::with_buffer_size(reader, DEFAULT_BUFFER_SIZE)
        }

        /// Create a new async stream reader with custom buffer size
        pub fn with_buffer_size(reader: R, buffer_size: usize) -> Self {
            Self {
                reader,
                buffer_size,
            }
        }

        /// Read all data and apply async transformation
        pub async fn read_all<F, Fut, T>(&mut self, mut transform: F) -> io::Result<Vec<T>>
        where
            F: FnMut(Vec<u8>) -> Fut,
            Fut: std::future::Future<Output = io::Result<T>>,
        {
            let mut results = Vec::new();
            let mut buffer = vec![0u8; self.buffer_size];

            loop {
                let n = self.reader.read(&mut buffer).await?;
                if n == 0 {
                    break;
                }
                let result = transform(buffer[..n].to_vec()).await?;
                results.push(result);
            }

            Ok(results)
        }

        /// Count bytes asynchronously
        pub async fn count_bytes(&mut self) -> io::Result<u64> {
            let mut total = 0u64;
            let mut buffer = vec![0u8; self.buffer_size];

            loop {
                let n = self.reader.read(&mut buffer).await?;
                if n == 0 {
                    break;
                }
                total += n as u64;
            }

            Ok(total)
        }
    }

    /// Async stream writer
    pub struct AsyncStreamWriter<W> {
        writer: W,
        buffer: Vec<u8>,
        buffer_size: usize,
    }

    impl<W: AsyncWriteExt + Unpin> AsyncStreamWriter<W> {
        /// Create a new async stream writer
        pub fn new(writer: W) -> Self {
            Self::with_buffer_size(writer, DEFAULT_BUFFER_SIZE)
        }

        /// Create a new async stream writer with custom buffer size
        pub fn with_buffer_size(writer: W, buffer_size: usize) -> Self {
            Self {
                writer,
                buffer: Vec::with_capacity(buffer_size),
                buffer_size,
            }
        }

        /// Write a chunk asynchronously
        pub async fn write_chunk(&mut self, data: &[u8]) -> io::Result<()> {
            if self.buffer.len() + data.len() <= self.buffer_size {
                self.buffer.extend_from_slice(data);
                return Ok(());
            }

            if !self.buffer.is_empty() {
                self.writer.write_all(&self.buffer).await?;
                self.buffer.clear();
            }

            if data.len() > self.buffer_size {
                self.writer.write_all(data).await?;
            } else {
                self.buffer.extend_from_slice(data);
            }

            Ok(())
        }

        /// Flush asynchronously
        pub async fn flush(&mut self) -> io::Result<()> {
            if !self.buffer.is_empty() {
                self.writer.write_all(&self.buffer).await?;
                self.buffer.clear();
            }
            self.writer.flush().await
        }

        /// Finish writing asynchronously
        pub async fn finish(mut self) -> io::Result<W> {
            self.flush().await?;
            Ok(self.writer)
        }
    }

    /// Stream write to file asynchronously
    pub async fn stream_write_file<P, I, D>(path: P, chunks: I) -> io::Result<()>
    where
        P: AsRef<Path>,
        I: Iterator<Item = D>,
        D: AsRef<[u8]>,
    {
        let file = tokio::fs::File::create(path).await?;
        let mut writer = AsyncStreamWriter::new(file);

        for chunk in chunks {
            writer.write_chunk(chunk.as_ref()).await?;
        }

        writer.flush().await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_stream_reader_count_bytes() {
        let data = b"Hello, world!";
        let cursor = Cursor::new(data);
        let mut reader = StreamReader::new(cursor);

        let count = reader.count_bytes().unwrap();
        assert_eq!(count, data.len() as u64);
    }

    #[test]
    fn test_stream_writer() {
        let mut buffer = Vec::new();
        let mut writer = StreamWriter::with_buffer_size(&mut buffer, 10);

        writer.write_chunk(b"Hello").unwrap();
        writer.write_chunk(b", ").unwrap();
        writer.write_chunk(b"world!").unwrap();
        writer.flush().unwrap();

        assert_eq!(buffer, b"Hello, world!");
    }

    #[test]
    fn test_stream_reader_fold() {
        let data = b"abcdefghij";
        let cursor = Cursor::new(data);
        let mut reader = StreamReader::with_buffer_size(cursor, 3);

        let result = reader.fold(0, |acc, chunk| Ok(acc + chunk.len())).unwrap();
        assert_eq!(result, data.len());
    }
}
