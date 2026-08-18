//! Hybrid Journaling System for Durable Writes
//! ============================================
//!
//! This module provides a combination of simple fsync barriers for atomic single-file
//! operations and a lightweight Write-Ahead Log (WAL) for complex multi-file transactions.
//!
//! # Why a Journal?
//!
//! Filesystems need journals to survive crashes. Without one, a power failure during
//! a write operation can leave the filesystem in an inconsistent state:
//!
//! - **Torn writes**: Half-written data appears as corruption
//! - **Lost updates**: Data in memory never reaches disk
//! - **Dangling references**: Metadata points to freed blocks
//! - **Orphaned data**: Blocks allocated but not referenced
//!
//! The journal ensures that either:
//! 1. An operation completes fully and is recoverable, OR
//! 2. An operation is completely rolled back (never partially applied)
//!
//! This is the "A" in ACID: Atomicity.
//!
//! # Why Hybrid (fsync + WAL)?
//!
//! A pure WAL approach (like SQLite or PostgreSQL) adds latency to every write.
//! A pure fsync approach can't handle multi-file transactions atomically.
//!
//! We use a **hybrid** approach optimized for the engram use case:
//!
//! | Operation Type | Strategy | Why |
//! |----------------|----------|-----|
//! | Single file < 64KB | fsync barrier | Simple, fast, no journal overhead |
//! | Single file ≥ 64KB | WAL | Allows streaming writes + recovery |
//! | Multi-file transaction | WAL | Atomic commit of all changes |
//! | Metadata-only change | WAL | Small payload, fast |
//!
//! This hybrid approach gives us:
//! - **Fast common case**: Single small files use simple fsync (~1ms)
//! - **Safe complex case**: Multi-file ops get full WAL protection
//! - **Bounded recovery time**: Journal is checkpointed regularly
//!
//! # Why These Durability Modes?
//!
//! Different users have different needs:
//!
//! ## Immediate Mode
//! - **Use when**: Data loss is unacceptable (financial, medical)
//! - **Cost**: ~5-10ms per write (SSD), ~15-30ms (HDD)
//! - **Guarantee**: Committed = on persistent storage
//!
//! ## GroupCommit Mode (Default)
//! - **Use when**: Balance of safety and performance
//! - **Cost**: ~5ms batch latency, amortized across many writes
//! - **Guarantee**: Up to 5ms of data loss on crash
//! - **Why default**: Most users want good performance with reasonable safety
//!
//! ## Relaxed Mode  
//! - **Use when**: Performance critical, data is regenerable
//! - **Cost**: Near-zero latency
//! - **Guarantee**: OS decides when to flush (could be 30+ seconds)
//! - **Risk**: Significant data loss on crash
//!
//! # Journal Record Format
//!
//! Why this specific format:
//!
//! ```text
//! ┌────────────────────────────────────────────────────────────┐
//! │ Record Header (32 bytes)                                   │
//! ├────────────────────────────────────────────────────────────┤
//! │ magic: u32          = 0x454D4252 ("EMBR")                  │
//! │   └─ Why: Detect corruption/wrong file immediately        │
//! │ version: u16        = 1                                    │
//! │   └─ Why: Allow format evolution without breaking old files│
//! │ flags: u16          = (committed | checkpointed | ...)     │
//! │   └─ Why: Track transaction state for recovery            │
//! │ txn_id: u64         = transaction ID                       │
//! │   └─ Why: Ordering for replay, debugging                  │
//! │ timestamp: u64      = Unix timestamp (nanos)               │
//! │   └─ Why: Debugging, determining recovery point           │
//! │ payload_len: u32    = length of payload                    │
//! │   └─ Why: Know how much to read without parsing payload   │
//! │ checksum: u32       = CRC32 of header + payload            │
//! │   └─ Why: Detect corruption before acting on bad data     │
//! ├────────────────────────────────────────────────────────────┤
//! │ Payload (variable)                                         │
//! │ - Serialized operations                                    │
//! │   └─ Why: Variable length allows efficient small writes   │
//! └────────────────────────────────────────────────────────────┘
//! ```
//!
//! # Why 32-byte Header?
//!
//! - **Alignment**: 32 bytes aligns to cache lines on most CPUs
//! - **Atomic read**: Can read header in single I/O operation
//! - **Space efficient**: Minimal overhead per record
//! - **Room for growth**: Version field allows adding fields later
//!
//! # Recovery Process
//!
//! On mount after crash:
//!
//! 1. **Scan journal** from start to end
//! 2. **Verify checksums** - skip corrupted records
//! 3. **Find committed transactions** - ignore pending/aborted
//! 4. **Replay in order** - apply operations to engram
//! 5. **Verify engram** - check manifest and chunk integrity
//! 6. **Checkpoint journal** - truncate replayed records
//!
//! Recovery is O(journal_size), typically <1 second for <100MB journal.
//!
//! # Durability Modes
//!
//! - `Immediate`: fsync after every write (safest, slowest)
//! - `GroupCommit`: batch fsync every N ms or M operations
//! - `Relaxed`: OS-managed flush (fastest, potential data loss on crash)

use std::collections::VecDeque;
use std::fs::{File, OpenOptions};
use std::io::{self, BufWriter, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use super::types::ChunkId;

/// Journal magic number: "EMBR" in little-endian
const JOURNAL_MAGIC: u32 = 0x52424D45;

/// Current journal format version
const JOURNAL_VERSION: u16 = 1;

/// Header size in bytes
const HEADER_SIZE: usize = 32;

/// Default group commit interval (5ms)
const DEFAULT_GROUP_COMMIT_MS: u64 = 5;

/// Default max pending operations before flush
const DEFAULT_MAX_PENDING_OPS: usize = 100;

/// Threshold for using WAL vs simple fsync (64KB)
const WAL_THRESHOLD_BYTES: usize = 64 * 1024;

/// Record flags
#[repr(u16)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RecordFlags {
    /// Transaction is pending (written but not committed)
    Pending = 0,
    /// Transaction is committed
    Committed = 1,
    /// Transaction has been checkpointed (can be truncated)
    Checkpointed = 2,
    /// Transaction was aborted
    Aborted = 3,
}

impl RecordFlags {
    fn from_u16(v: u16) -> Option<Self> {
        match v {
            0 => Some(Self::Pending),
            1 => Some(Self::Committed),
            2 => Some(Self::Checkpointed),
            3 => Some(Self::Aborted),
            _ => None,
        }
    }
}

/// Durability mode for journal writes
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DurabilityMode {
    /// fsync after every write (safest)
    Immediate,
    /// Batch fsync with configurable interval/count
    GroupCommit {
        max_delay_ms: u64,
        max_pending_ops: usize,
    },
    /// OS-managed flush (fastest, potential data loss)
    Relaxed,
}

impl Default for DurabilityMode {
    fn default() -> Self {
        // Default to group commit for balance of safety and performance
        Self::GroupCommit {
            max_delay_ms: DEFAULT_GROUP_COMMIT_MS,
            max_pending_ops: DEFAULT_MAX_PENDING_OPS,
        }
    }
}

/// Journal record header
#[derive(Clone, Debug)]
pub struct RecordHeader {
    pub magic: u32,
    pub version: u16,
    pub flags: RecordFlags,
    pub txn_id: u64,
    pub timestamp: u64,
    pub payload_len: u32,
    pub checksum: u32,
}

impl RecordHeader {
    /// Create a new record header
    pub fn new(txn_id: u64, flags: RecordFlags, payload_len: u32) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos() as u64;

        Self {
            magic: JOURNAL_MAGIC,
            version: JOURNAL_VERSION,
            flags,
            txn_id,
            timestamp,
            payload_len,
            checksum: 0, // Computed on serialize
        }
    }

    /// Serialize header to bytes
    pub fn to_bytes(&self, payload: &[u8]) -> Vec<u8> {
        let mut buf = Vec::with_capacity(HEADER_SIZE);

        buf.extend_from_slice(&self.magic.to_le_bytes());
        buf.extend_from_slice(&self.version.to_le_bytes());
        buf.extend_from_slice(&(self.flags as u16).to_le_bytes());
        buf.extend_from_slice(&self.txn_id.to_le_bytes());
        buf.extend_from_slice(&self.timestamp.to_le_bytes());
        buf.extend_from_slice(&self.payload_len.to_le_bytes());

        // Compute checksum over header (without checksum field) + payload
        let checksum = crc32_compute(&buf, payload);
        buf.extend_from_slice(&checksum.to_le_bytes());

        buf
    }

    /// Parse header from bytes
    pub fn from_bytes(data: &[u8]) -> io::Result<Self> {
        if data.len() < HEADER_SIZE {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "header too short",
            ));
        }

        let magic = u32::from_le_bytes(data[0..4].try_into().unwrap());
        if magic != JOURNAL_MAGIC {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("invalid magic: {:08x}", magic),
            ));
        }

        let version = u16::from_le_bytes(data[4..6].try_into().unwrap());
        if version != JOURNAL_VERSION {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("unsupported version: {}", version),
            ));
        }

        let flags = RecordFlags::from_u16(u16::from_le_bytes(data[6..8].try_into().unwrap()))
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "invalid flags"))?;

        let txn_id = u64::from_le_bytes(data[8..16].try_into().unwrap());
        let timestamp = u64::from_le_bytes(data[16..24].try_into().unwrap());
        let payload_len = u32::from_le_bytes(data[24..28].try_into().unwrap());
        let checksum = u32::from_le_bytes(data[28..32].try_into().unwrap());

        Ok(Self {
            magic,
            version,
            flags,
            txn_id,
            timestamp,
            payload_len,
            checksum,
        })
    }

    /// Verify checksum against payload
    pub fn verify_checksum(&self, payload: &[u8]) -> bool {
        let mut header_bytes = Vec::with_capacity(28);
        header_bytes.extend_from_slice(&self.magic.to_le_bytes());
        header_bytes.extend_from_slice(&self.version.to_le_bytes());
        header_bytes.extend_from_slice(&(self.flags as u16).to_le_bytes());
        header_bytes.extend_from_slice(&self.txn_id.to_le_bytes());
        header_bytes.extend_from_slice(&self.timestamp.to_le_bytes());
        header_bytes.extend_from_slice(&self.payload_len.to_le_bytes());

        let computed = crc32_compute(&header_bytes, payload);
        computed == self.checksum
    }
}

/// Simple CRC32 computation (no external dependency)
fn crc32_compute(header: &[u8], payload: &[u8]) -> u32 {
    // CRC32-ISO polynomial
    const POLYNOMIAL: u32 = 0xEDB88320;

    let mut crc = 0xFFFFFFFF_u32;

    for &byte in header.iter().chain(payload.iter()) {
        crc ^= byte as u32;
        for _ in 0..8 {
            if crc & 1 != 0 {
                crc = (crc >> 1) ^ POLYNOMIAL;
            } else {
                crc >>= 1;
            }
        }
    }

    !crc
}

/// Serialized operation for the journal
#[derive(Clone, Debug)]
pub enum JournalOp {
    WriteChunk {
        chunk_id: ChunkId,
        data_hash: [u8; 32],
        data_len: usize,
    },
    DeleteChunk {
        chunk_id: ChunkId,
    },
    WriteFile {
        path: String,
        chunk_ids: Vec<ChunkId>,
        size: usize,
    },
    DeleteFile {
        path: String,
    },
    UpdateRoot {
        root_hash: [u8; 32],
    },
    Barrier,
}

impl JournalOp {
    /// Serialize operation to bytes
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut buf = Vec::new();

        match self {
            JournalOp::WriteChunk {
                chunk_id,
                data_hash,
                data_len,
            } => {
                buf.push(1);
                buf.extend_from_slice(&(*chunk_id as u64).to_le_bytes());
                buf.extend_from_slice(data_hash);
                buf.extend_from_slice(&(*data_len as u64).to_le_bytes());
            }
            JournalOp::DeleteChunk { chunk_id } => {
                buf.push(2);
                buf.extend_from_slice(&(*chunk_id as u64).to_le_bytes());
            }
            JournalOp::WriteFile {
                path,
                chunk_ids,
                size,
            } => {
                buf.push(3);
                let path_bytes = path.as_bytes();
                buf.extend_from_slice(&(path_bytes.len() as u32).to_le_bytes());
                buf.extend_from_slice(path_bytes);
                buf.extend_from_slice(&(chunk_ids.len() as u32).to_le_bytes());
                for &id in chunk_ids {
                    buf.extend_from_slice(&(id as u64).to_le_bytes());
                }
                buf.extend_from_slice(&(*size as u64).to_le_bytes());
            }
            JournalOp::DeleteFile { path } => {
                buf.push(4);
                let path_bytes = path.as_bytes();
                buf.extend_from_slice(&(path_bytes.len() as u32).to_le_bytes());
                buf.extend_from_slice(path_bytes);
            }
            JournalOp::UpdateRoot { root_hash } => {
                buf.push(5);
                buf.extend_from_slice(root_hash);
            }
            JournalOp::Barrier => {
                buf.push(6);
            }
        }

        buf
    }

    /// Deserialize operation from bytes
    pub fn from_bytes(data: &[u8]) -> io::Result<(Self, usize)> {
        if data.is_empty() {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "empty data"));
        }

        let op_type = data[0];
        let mut pos = 1;

        let op = match op_type {
            1 => {
                // WriteChunk
                if data.len() < pos + 8 + 32 + 8 {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "truncated WriteChunk",
                    ));
                }
                let chunk_id =
                    u64::from_le_bytes(data[pos..pos + 8].try_into().unwrap()) as ChunkId;
                pos += 8;
                let mut data_hash = [0u8; 32];
                data_hash.copy_from_slice(&data[pos..pos + 32]);
                pos += 32;
                let data_len = u64::from_le_bytes(data[pos..pos + 8].try_into().unwrap()) as usize;
                pos += 8;
                JournalOp::WriteChunk {
                    chunk_id,
                    data_hash,
                    data_len,
                }
            }
            2 => {
                // DeleteChunk
                if data.len() < pos + 8 {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "truncated DeleteChunk",
                    ));
                }
                let chunk_id =
                    u64::from_le_bytes(data[pos..pos + 8].try_into().unwrap()) as ChunkId;
                pos += 8;
                JournalOp::DeleteChunk { chunk_id }
            }
            3 => {
                // WriteFile
                if data.len() < pos + 4 {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "truncated WriteFile path_len",
                    ));
                }
                let path_len = u32::from_le_bytes(data[pos..pos + 4].try_into().unwrap()) as usize;
                pos += 4;
                if data.len() < pos + path_len {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "truncated WriteFile path",
                    ));
                }
                let path = String::from_utf8_lossy(&data[pos..pos + path_len]).into_owned();
                pos += path_len;

                if data.len() < pos + 4 {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "truncated WriteFile chunk_count",
                    ));
                }
                let chunk_count =
                    u32::from_le_bytes(data[pos..pos + 4].try_into().unwrap()) as usize;
                pos += 4;

                let mut chunk_ids = Vec::with_capacity(chunk_count);
                for _ in 0..chunk_count {
                    if data.len() < pos + 8 {
                        return Err(io::Error::new(
                            io::ErrorKind::InvalidData,
                            "truncated WriteFile chunk_id",
                        ));
                    }
                    let id = u64::from_le_bytes(data[pos..pos + 8].try_into().unwrap()) as ChunkId;
                    pos += 8;
                    chunk_ids.push(id);
                }

                if data.len() < pos + 8 {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "truncated WriteFile size",
                    ));
                }
                let size = u64::from_le_bytes(data[pos..pos + 8].try_into().unwrap()) as usize;
                pos += 8;

                JournalOp::WriteFile {
                    path,
                    chunk_ids,
                    size,
                }
            }
            4 => {
                // DeleteFile
                if data.len() < pos + 4 {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "truncated DeleteFile",
                    ));
                }
                let path_len = u32::from_le_bytes(data[pos..pos + 4].try_into().unwrap()) as usize;
                pos += 4;
                if data.len() < pos + path_len {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "truncated DeleteFile path",
                    ));
                }
                let path = String::from_utf8_lossy(&data[pos..pos + path_len]).into_owned();
                pos += path_len;
                JournalOp::DeleteFile { path }
            }
            5 => {
                // UpdateRoot
                if data.len() < pos + 32 {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "truncated UpdateRoot",
                    ));
                }
                let mut root_hash = [0u8; 32];
                root_hash.copy_from_slice(&data[pos..pos + 32]);
                pos += 32;
                JournalOp::UpdateRoot { root_hash }
            }
            6 => JournalOp::Barrier,
            _ => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("unknown op type: {}", op_type),
                ))
            }
        };

        Ok((op, pos))
    }
}

/// A pending write waiting for group commit
struct PendingWrite {
    #[allow(dead_code)]
    txn_id: u64,
    data: Vec<u8>,
    committed: Arc<(Mutex<bool>, Condvar)>,
}

/// Journal for durable writes
pub struct Journal {
    /// Path to journal file
    path: PathBuf,
    /// Durability mode
    mode: DurabilityMode,
    /// Journal file writer (protected by mutex for group commit)
    writer: Mutex<BufWriter<File>>,
    /// Next transaction ID
    next_txn_id: AtomicU64,
    /// Whether journal is open
    open: AtomicBool,
    /// Pending writes for group commit
    pending: Mutex<VecDeque<PendingWrite>>,
    /// Last flush time
    last_flush: Mutex<Instant>,
    /// Background flush thread handle
    flush_thread: Mutex<Option<JoinHandle<()>>>,
    /// Signal to stop flush thread
    stop_signal: Arc<AtomicBool>,
}

impl Journal {
    /// Create or open a journal at the given path
    pub fn open(path: impl AsRef<Path>, mode: DurabilityMode) -> io::Result<Arc<Self>> {
        let path = path.as_ref().to_path_buf();

        // Open or create journal file
        let file = OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(&path)?;

        let journal = Arc::new(Self {
            path,
            mode,
            writer: Mutex::new(BufWriter::new(file)),
            next_txn_id: AtomicU64::new(1),
            open: AtomicBool::new(true),
            pending: Mutex::new(VecDeque::new()),
            last_flush: Mutex::new(Instant::now()),
            flush_thread: Mutex::new(None),
            stop_signal: Arc::new(AtomicBool::new(false)),
        });

        // Start background flush thread for group commit mode
        if let DurabilityMode::GroupCommit { max_delay_ms, .. } = mode {
            let journal_clone = Arc::clone(&journal);
            let stop = Arc::clone(&journal.stop_signal);

            let handle = thread::spawn(move || {
                while !stop.load(Ordering::Relaxed) {
                    thread::sleep(Duration::from_millis(max_delay_ms));
                    if !stop.load(Ordering::Relaxed) {
                        let _ = journal_clone.flush_pending();
                    }
                }
            });

            *journal.flush_thread.lock().unwrap() = Some(handle);
        }

        Ok(journal)
    }

    /// Write a transaction record to the journal
    pub fn write_transaction(&self, ops: &[JournalOp]) -> io::Result<u64> {
        if !self.open.load(Ordering::Acquire) {
            return Err(io::Error::other("journal closed"));
        }

        let txn_id = self.next_txn_id.fetch_add(1, Ordering::AcqRel);

        // Serialize operations
        let mut payload = Vec::new();
        for op in ops {
            payload.extend_from_slice(&op.to_bytes());
        }

        // Create record
        let header = RecordHeader::new(txn_id, RecordFlags::Committed, payload.len() as u32);
        let header_bytes = header.to_bytes(&payload);

        match self.mode {
            DurabilityMode::Immediate => {
                // Write and fsync immediately
                let mut writer = self.writer.lock().unwrap();
                writer.write_all(&header_bytes)?;
                writer.write_all(&payload)?;
                writer.flush()?;
                writer.get_ref().sync_all()?;
            }
            DurabilityMode::GroupCommit {
                max_pending_ops, ..
            } => {
                // Add to pending queue
                let committed = Arc::new((Mutex::new(false), Condvar::new()));
                let committed_clone = Arc::clone(&committed);

                let mut record_data = header_bytes;
                record_data.extend_from_slice(&payload);

                {
                    let mut pending = self.pending.lock().unwrap();
                    pending.push_back(PendingWrite {
                        txn_id,
                        data: record_data,
                        committed: committed_clone,
                    });

                    // Flush if we've hit the pending limit
                    if pending.len() >= max_pending_ops {
                        drop(pending);
                        self.flush_pending()?;
                    }
                }

                // Wait for commit
                let (lock, cvar) = &*committed;
                let mut done = lock.lock().unwrap();
                while !*done {
                    done = cvar.wait(done).unwrap();
                }
            }
            DurabilityMode::Relaxed => {
                // Write without fsync
                let mut writer = self.writer.lock().unwrap();
                writer.write_all(&header_bytes)?;
                writer.write_all(&payload)?;
                // Don't fsync - let OS handle it
            }
        }

        Ok(txn_id)
    }

    /// Flush all pending writes to disk
    pub fn flush_pending(&self) -> io::Result<()> {
        let writes: Vec<PendingWrite> = {
            let mut pending = self.pending.lock().unwrap();
            pending.drain(..).collect()
        };

        if writes.is_empty() {
            return Ok(());
        }

        // Write all pending records
        {
            let mut writer = self.writer.lock().unwrap();
            for write in &writes {
                writer.write_all(&write.data)?;
            }
            writer.flush()?;
            writer.get_ref().sync_all()?;
        }

        // Signal all waiters
        for write in writes {
            let (lock, cvar) = &*write.committed;
            let mut done = lock.lock().unwrap();
            *done = true;
            cvar.notify_one();
        }

        *self.last_flush.lock().unwrap() = Instant::now();

        Ok(())
    }

    /// Write a simple fsync barrier (for small single-file ops)
    pub fn write_barrier(&self) -> io::Result<()> {
        self.write_transaction(&[JournalOp::Barrier])?;
        Ok(())
    }

    /// Read and replay journal records
    pub fn replay(&self) -> io::Result<Vec<(u64, Vec<JournalOp>)>> {
        let mut file = OpenOptions::new().read(true).open(&self.path)?;
        file.seek(SeekFrom::Start(0))?;

        let mut transactions = Vec::new();
        let mut header_buf = [0u8; HEADER_SIZE];

        loop {
            match file.read_exact(&mut header_buf) {
                Ok(()) => {}
                Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => break,
                Err(e) => return Err(e),
            }

            let header = RecordHeader::from_bytes(&header_buf)?;

            // Read payload
            let mut payload = vec![0u8; header.payload_len as usize];
            file.read_exact(&mut payload)?;

            // Verify checksum
            if !header.verify_checksum(&payload) {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "checksum mismatch",
                ));
            }

            // Skip non-committed records
            if header.flags != RecordFlags::Committed {
                continue;
            }

            // Parse operations
            let mut ops = Vec::new();
            let mut pos = 0;
            while pos < payload.len() {
                let (op, consumed) = JournalOp::from_bytes(&payload[pos..])?;
                ops.push(op);
                pos += consumed;
            }

            transactions.push((header.txn_id, ops));
        }

        Ok(transactions)
    }

    /// Checkpoint the journal (truncate replayed records)
    pub fn checkpoint(&self) -> io::Result<()> {
        // Flush any pending writes first
        self.flush_pending()?;

        // Truncate the journal file
        let mut writer = self.writer.lock().unwrap();
        writer.get_ref().set_len(0)?;
        writer.seek(SeekFrom::Start(0))?;
        writer.get_ref().sync_all()?;

        Ok(())
    }

    /// Close the journal
    pub fn close(&self) -> io::Result<()> {
        self.open.store(false, Ordering::Release);
        self.stop_signal.store(true, Ordering::Release);

        // Flush remaining writes
        self.flush_pending()?;

        // Wait for flush thread to finish
        if let Some(handle) = self.flush_thread.lock().unwrap().take() {
            let _ = handle.join();
        }

        Ok(())
    }

    /// Check if an operation should use WAL vs simple barrier
    pub fn should_use_wal(data_size: usize, op_count: usize) -> bool {
        data_size > WAL_THRESHOLD_BYTES || op_count > 1
    }
}

impl Drop for Journal {
    fn drop(&mut self) {
        let _ = self.close();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_record_header_roundtrip() {
        let payload = b"test payload data";
        let header = RecordHeader::new(42, RecordFlags::Committed, payload.len() as u32);

        let header_bytes = header.to_bytes(payload);
        let parsed = RecordHeader::from_bytes(&header_bytes).unwrap();

        assert_eq!(parsed.magic, JOURNAL_MAGIC);
        assert_eq!(parsed.version, JOURNAL_VERSION);
        assert_eq!(parsed.flags, RecordFlags::Committed);
        assert_eq!(parsed.txn_id, 42);
        assert_eq!(parsed.payload_len, payload.len() as u32);
        assert!(parsed.verify_checksum(payload));
    }

    #[test]
    fn test_journal_op_roundtrip() {
        let ops = vec![
            JournalOp::WriteChunk {
                chunk_id: 123,
                data_hash: [0xAB; 32],
                data_len: 4096,
            },
            JournalOp::WriteFile {
                path: "/etc/passwd".to_string(),
                chunk_ids: vec![1, 2, 3],
                size: 1024,
            },
            JournalOp::DeleteFile {
                path: "/tmp/test".to_string(),
            },
            JournalOp::Barrier,
        ];

        for op in ops {
            let bytes = op.to_bytes();
            let (parsed, _) = JournalOp::from_bytes(&bytes).unwrap();

            match (&op, &parsed) {
                (
                    JournalOp::WriteChunk {
                        chunk_id: a,
                        data_len: b,
                        ..
                    },
                    JournalOp::WriteChunk {
                        chunk_id: c,
                        data_len: d,
                        ..
                    },
                ) => {
                    assert_eq!(a, c);
                    assert_eq!(b, d);
                }
                (
                    JournalOp::WriteFile {
                        path: a,
                        chunk_ids: b,
                        ..
                    },
                    JournalOp::WriteFile {
                        path: c,
                        chunk_ids: d,
                        ..
                    },
                ) => {
                    assert_eq!(a, c);
                    assert_eq!(b, d);
                }
                (JournalOp::DeleteFile { path: a }, JournalOp::DeleteFile { path: b }) => {
                    assert_eq!(a, b);
                }
                (JournalOp::Barrier, JournalOp::Barrier) => {}
                _ => panic!("mismatched op types"),
            }
        }
    }

    #[test]
    fn test_journal_write_replay() {
        let dir = tempdir().unwrap();
        let journal_path = dir.path().join("test.journal");

        // Write some transactions
        {
            let journal = Journal::open(&journal_path, DurabilityMode::Immediate).unwrap();

            journal
                .write_transaction(&[JournalOp::WriteChunk {
                    chunk_id: 1,
                    data_hash: [0x11; 32],
                    data_len: 100,
                }])
                .unwrap();

            journal
                .write_transaction(&[
                    JournalOp::WriteFile {
                        path: "/test".to_string(),
                        chunk_ids: vec![1],
                        size: 100,
                    },
                    JournalOp::Barrier,
                ])
                .unwrap();

            journal.close().unwrap();
        }

        // Replay
        {
            let journal = Journal::open(&journal_path, DurabilityMode::Immediate).unwrap();
            let txns = journal.replay().unwrap();

            assert_eq!(txns.len(), 2);
            assert_eq!(txns[0].0, 1); // txn_id
            assert_eq!(txns[1].0, 2);

            // First transaction has 1 op
            assert_eq!(txns[0].1.len(), 1);
            // Second transaction has 2 ops
            assert_eq!(txns[1].1.len(), 2);
        }
    }

    #[test]
    fn test_should_use_wal() {
        // Small single op - no WAL
        assert!(!Journal::should_use_wal(1024, 1));

        // Large single op - use WAL
        assert!(Journal::should_use_wal(100 * 1024, 1));

        // Multiple ops - use WAL
        assert!(Journal::should_use_wal(1024, 5));
    }
}
