//! Serialization and deserialization utilities
//!
//! Provides high-level interfaces for encoding/decoding data in various formats:
//! - Bincode (binary, efficient)
//! - JSON (text, human-readable)
//!
//! Both sync and async variants are available when the `async` feature is enabled.

use std::io::{self, Read, Write};
use std::path::Path;

/// Serialize data to bincode format
///
/// # Examples
/// ```
/// use embeddenator_io::to_bincode;
/// use serde::{Serialize, Deserialize};
///
/// #[derive(Serialize, Deserialize, PartialEq, Debug)]
/// struct Data { value: u32 }
///
/// let data = Data { value: 42 };
/// let bytes = to_bincode(&data).unwrap();
/// assert!(!bytes.is_empty());
/// ```
pub fn to_bincode<T: serde::Serialize>(value: &T) -> io::Result<Vec<u8>> {
    bincode::serialize(value).map_err(io::Error::other)
}

/// Deserialize data from bincode format
///
/// # Examples
/// ```
/// use embeddenator_io::{to_bincode, from_bincode};
/// use serde::{Serialize, Deserialize};
///
/// #[derive(Serialize, Deserialize, PartialEq, Debug)]
/// struct Data { value: u32 }
///
/// let data = Data { value: 42 };
/// let bytes = to_bincode(&data).unwrap();
/// let decoded: Data = from_bincode(&bytes).unwrap();
/// assert_eq!(data, decoded);
/// ```
pub fn from_bincode<T: serde::de::DeserializeOwned>(bytes: &[u8]) -> io::Result<T> {
    bincode::deserialize(bytes).map_err(io::Error::other)
}

/// Serialize data to JSON format (pretty-printed)
///
/// # Examples
/// ```
/// use embeddenator_io::to_json_pretty;
/// use serde::{Serialize, Deserialize};
///
/// #[derive(Serialize, Deserialize)]
/// struct Data { value: u32 }
///
/// let data = Data { value: 42 };
/// let json = to_json_pretty(&data).unwrap();
/// assert!(json.contains("value"));
/// ```
pub fn to_json_pretty<T: serde::Serialize>(value: &T) -> io::Result<String> {
    serde_json::to_string_pretty(value).map_err(io::Error::other)
}

/// Serialize data to JSON format (compact)
pub fn to_json<T: serde::Serialize>(value: &T) -> io::Result<String> {
    serde_json::to_string(value).map_err(io::Error::other)
}

/// Deserialize data from JSON format
///
/// # Examples
/// ```
/// use embeddenator_io::{to_json, from_json};
/// use serde::{Serialize, Deserialize};
///
/// #[derive(Serialize, Deserialize, PartialEq, Debug)]
/// struct Data { value: u32 }
///
/// let data = Data { value: 42 };
/// let json = to_json(&data).unwrap();
/// let decoded: Data = from_json(&json).unwrap();
/// assert_eq!(data, decoded);
/// ```
pub fn from_json<T: serde::de::DeserializeOwned>(json: &str) -> io::Result<T> {
    serde_json::from_str(json).map_err(io::Error::other)
}

/// Write data to a file in bincode format
///
/// # Examples
/// ```no_run
/// use embeddenator_io::write_bincode_file;
/// use serde::Serialize;
///
/// #[derive(Serialize)]
/// struct Data { value: u32 }
///
/// let data = Data { value: 42 };
/// write_bincode_file("data.bin", &data).unwrap();
/// ```
pub fn write_bincode_file<P: AsRef<Path>, T: serde::Serialize>(
    path: P,
    value: &T,
) -> io::Result<()> {
    let bytes = to_bincode(value)?;
    std::fs::write(path, bytes)
}

/// Read data from a file in bincode format
///
/// # Examples
/// ```no_run
/// use embeddenator_io::{write_bincode_file, read_bincode_file};
/// use serde::{Serialize, Deserialize};
///
/// #[derive(Serialize, Deserialize, PartialEq, Debug)]
/// struct Data { value: u32 }
///
/// let data = Data { value: 42 };
/// write_bincode_file("data.bin", &data).unwrap();
/// let loaded: Data = read_bincode_file("data.bin").unwrap();
/// assert_eq!(data, loaded);
/// ```
pub fn read_bincode_file<P: AsRef<Path>, T: serde::de::DeserializeOwned>(path: P) -> io::Result<T> {
    let bytes = std::fs::read(path)?;
    from_bincode(&bytes)
}

/// Write data to a file in JSON format (pretty-printed)
///
/// # Examples
/// ```no_run
/// use embeddenator_io::write_json_file;
/// use serde::Serialize;
///
/// #[derive(Serialize)]
/// struct Data { value: u32 }
///
/// let data = Data { value: 42 };
/// write_json_file("data.json", &data).unwrap();
/// ```
pub fn write_json_file<P: AsRef<Path>, T: serde::Serialize>(path: P, value: &T) -> io::Result<()> {
    let file = std::fs::File::create(path)?;
    serde_json::to_writer_pretty(file, value).map_err(io::Error::other)
}

/// Read data from a file in JSON format
///
/// # Examples
/// ```no_run
/// use embeddenator_io::{write_json_file, read_json_file};
/// use serde::{Serialize, Deserialize};
///
/// #[derive(Serialize, Deserialize, PartialEq, Debug)]
/// struct Data { value: u32 }
///
/// let data = Data { value: 42 };
/// write_json_file("data.json", &data).unwrap();
/// let loaded: Data = read_json_file("data.json").unwrap();
/// assert_eq!(data.value, loaded.value);
/// ```
pub fn read_json_file<P: AsRef<Path>, T: serde::de::DeserializeOwned>(path: P) -> io::Result<T> {
    let file = std::fs::File::open(path)?;
    serde_json::from_reader(file).map_err(io::Error::other)
}

/// Write data to a writer in bincode format
pub fn write_bincode<W: Write, T: serde::Serialize>(writer: &mut W, value: &T) -> io::Result<()> {
    let bytes = to_bincode(value)?;
    writer.write_all(&bytes)
}

/// Read data from a reader in bincode format
pub fn read_bincode<R: Read, T: serde::de::DeserializeOwned>(reader: &mut R) -> io::Result<T> {
    let mut bytes = Vec::new();
    reader.read_to_end(&mut bytes)?;
    from_bincode(&bytes)
}

/// Write data to a writer in JSON format (pretty-printed)
pub fn write_json_pretty<W: Write, T: serde::Serialize>(
    writer: &mut W,
    value: &T,
) -> io::Result<()> {
    serde_json::to_writer_pretty(writer, value).map_err(io::Error::other)
}

/// Write data to a writer in JSON format (compact)
pub fn write_json_compact<W: Write, T: serde::Serialize>(
    writer: &mut W,
    value: &T,
) -> io::Result<()> {
    serde_json::to_writer(writer, value).map_err(io::Error::other)
}

/// Read data from a reader in JSON format
pub fn read_json<R: Read, T: serde::de::DeserializeOwned>(reader: &mut R) -> io::Result<T> {
    serde_json::from_reader(reader).map_err(io::Error::other)
}

#[cfg(feature = "async")]
pub mod async_serialize {
    //! Async variants of serialization functions

    use std::io;
    use std::path::Path;
    use tokio::io::AsyncWriteExt;

    /// Write data to a file in bincode format (async)
    pub async fn write_bincode_file<P: AsRef<Path>, T: serde::Serialize>(
        path: P,
        value: &T,
    ) -> io::Result<()> {
        let bytes = super::to_bincode(value)?;
        tokio::fs::write(path, bytes).await
    }

    /// Read data from a file in bincode format (async)
    pub async fn read_bincode_file<P: AsRef<Path>, T: serde::de::DeserializeOwned>(
        path: P,
    ) -> io::Result<T> {
        let bytes = tokio::fs::read(path).await?;
        super::from_bincode(&bytes)
    }

    /// Write data to a file in JSON format (async)
    pub async fn write_json_file<P: AsRef<Path>, T: serde::Serialize>(
        path: P,
        value: &T,
    ) -> io::Result<()> {
        let json = super::to_json_pretty(value)?;
        tokio::fs::write(path, json.as_bytes()).await
    }

    /// Read data from a file in JSON format (async)
    pub async fn read_json_file<P: AsRef<Path>, T: serde::de::DeserializeOwned>(
        path: P,
    ) -> io::Result<T> {
        let bytes = tokio::fs::read(path).await?;
        let json = String::from_utf8(bytes).map_err(io::Error::other)?;
        super::from_json(&json)
    }

    /// Write bincode data to an async writer
    pub async fn write_bincode<W: AsyncWriteExt + Unpin, T: serde::Serialize>(
        writer: &mut W,
        value: &T,
    ) -> io::Result<()> {
        let bytes = super::to_bincode(value)?;
        writer.write_all(&bytes).await
    }

    /// Write JSON data to an async writer (pretty)
    pub async fn write_json_pretty<W: AsyncWriteExt + Unpin, T: serde::Serialize>(
        writer: &mut W,
        value: &T,
    ) -> io::Result<()> {
        let json = super::to_json_pretty(value)?;
        writer.write_all(json.as_bytes()).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};

    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    struct TestData {
        id: u32,
        name: String,
        values: Vec<i32>,
    }

    impl TestData {
        fn sample() -> Self {
            TestData {
                id: 42,
                name: "test".to_string(),
                values: vec![1, 2, 3],
            }
        }
    }

    #[test]
    fn test_bincode_roundtrip() {
        let data = TestData::sample();
        let bytes = to_bincode(&data).unwrap();
        let decoded: TestData = from_bincode(&bytes).unwrap();
        assert_eq!(data, decoded);
    }

    #[test]
    fn test_json_roundtrip() {
        let data = TestData::sample();
        let json = to_json(&data).unwrap();
        let decoded: TestData = from_json(&json).unwrap();
        assert_eq!(data, decoded);
    }

    #[test]
    fn test_json_pretty() {
        let data = TestData::sample();
        let json = to_json_pretty(&data).unwrap();
        assert!(json.contains('\n')); // Pretty format has newlines
        assert!(json.contains("  ")); // Pretty format has indentation
    }
}
