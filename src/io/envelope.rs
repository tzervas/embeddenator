use std::io;

const MAGIC: [u8; 4] = *b"EDN1";
const HEADER_LEN: usize = 16;

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PayloadKind {
    EngramBincode = 1,
    SubEngramBincode = 2,
}

impl PayloadKind {
    fn from_u8(v: u8) -> Option<Self> {
        match v {
            1 => Some(Self::EngramBincode),
            2 => Some(Self::SubEngramBincode),
            _ => None,
        }
    }
}

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CompressionCodec {
    None = 0,
    Zstd = 1,
    Lz4 = 2,
}

impl CompressionCodec {
    fn from_u8(v: u8) -> Option<Self> {
        match v {
            0 => Some(Self::None),
            1 => Some(Self::Zstd),
            2 => Some(Self::Lz4),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct BinaryWriteOptions {
    pub codec: CompressionCodec,
    pub level: Option<i32>,
}

impl Default for BinaryWriteOptions {
    fn default() -> Self {
        Self {
            codec: CompressionCodec::None,
            level: None,
        }
    }
}

pub fn wrap_or_legacy(kind: PayloadKind, opts: BinaryWriteOptions, raw: &[u8]) -> io::Result<Vec<u8>> {
    if opts.codec == CompressionCodec::None {
        return Ok(raw.to_vec());
    }

    let compressed = compress(opts.codec, raw, opts.level)?;

    let mut out = Vec::with_capacity(HEADER_LEN + compressed.len());
    out.extend_from_slice(&MAGIC);
    out.push(kind as u8);
    out.push(opts.codec as u8);
    out.extend_from_slice(&0u16.to_le_bytes());
    out.extend_from_slice(&(raw.len() as u64).to_le_bytes());
    out.extend_from_slice(&compressed);

    Ok(out)
}

pub fn unwrap_auto(expected_kind: PayloadKind, data: &[u8]) -> io::Result<Vec<u8>> {
    if data.len() < HEADER_LEN || data[..4] != MAGIC {
        return Ok(data.to_vec());
    }

    let kind = PayloadKind::from_u8(data[4]).ok_or_else(|| io::Error::other("unknown envelope payload kind"))?;
    if kind != expected_kind {
        return Err(io::Error::other("unexpected envelope payload kind"));
    }

    let codec = CompressionCodec::from_u8(data[5]).ok_or_else(|| io::Error::other("unknown envelope compression codec"))?;
    let uncompressed_len = u64::from_le_bytes(data[8..16].try_into().expect("slice length checked")) as usize;

    let payload = &data[HEADER_LEN..];
    let decoded = match codec {
        CompressionCodec::None => payload.to_vec(),
        CompressionCodec::Zstd | CompressionCodec::Lz4 => decompress(codec, payload)?,
    };

    if decoded.len() != uncompressed_len {
        return Err(io::Error::other("envelope size mismatch"));
    }

    Ok(decoded)
}

fn compress(codec: CompressionCodec, raw: &[u8], level: Option<i32>) -> io::Result<Vec<u8>> {
    match codec {
        CompressionCodec::None => Ok(raw.to_vec()),
        CompressionCodec::Zstd => compress_zstd(raw, level),
        CompressionCodec::Lz4 => compress_lz4(raw),
    }
}

fn decompress(codec: CompressionCodec, payload: &[u8]) -> io::Result<Vec<u8>> {
    match codec {
        CompressionCodec::None => Ok(payload.to_vec()),
        CompressionCodec::Zstd => decompress_zstd(payload),
        CompressionCodec::Lz4 => decompress_lz4(payload),
    }
}

fn compress_zstd(_raw: &[u8], _level: Option<i32>) -> io::Result<Vec<u8>> {
    #[cfg(feature = "compression-zstd")]
    {
        use std::io::Cursor;
        let lvl = _level.unwrap_or(0);
        return zstd::stream::encode_all(Cursor::new(_raw), lvl).map_err(io::Error::other);
    }

    #[cfg(not(feature = "compression-zstd"))]
    {
        Err(io::Error::other("zstd compression support not enabled (enable feature `compression-zstd`)"))
    }
}

fn decompress_zstd(_payload: &[u8]) -> io::Result<Vec<u8>> {
    #[cfg(feature = "compression-zstd")]
    {
        use std::io::Cursor;
        return zstd::stream::decode_all(Cursor::new(_payload)).map_err(io::Error::other);
    }

    #[cfg(not(feature = "compression-zstd"))]
    {
        Err(io::Error::other("zstd decompression support not enabled (enable feature `compression-zstd`)"))
    }
}

fn compress_lz4(_raw: &[u8]) -> io::Result<Vec<u8>> {
    #[cfg(feature = "compression-lz4")]
    {
        return Ok(lz4_flex::compress_prepend_size(_raw));
    }

    #[cfg(not(feature = "compression-lz4"))]
    {
        Err(io::Error::other("lz4 compression support not enabled (enable feature `compression-lz4`)"))
    }
}

fn decompress_lz4(_payload: &[u8]) -> io::Result<Vec<u8>> {
    #[cfg(feature = "compression-lz4")]
    {
        return lz4_flex::decompress_size_prepended(_payload).map_err(io::Error::other);
    }

    #[cfg(not(feature = "compression-lz4"))]
    {
        Err(io::Error::other("lz4 decompression support not enabled (enable feature `compression-lz4`)"))
    }
}
