//! Transaction support for atomic multi-operation updates
//!
//! Provides ACID-like properties for complex operations that span multiple
//! components (codebook, manifest, corrections).

use super::types::ChunkId;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

/// Transaction status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransactionStatus {
    /// Transaction is in progress
    Pending,
    /// Transaction completed successfully
    Committed,
    /// Transaction was rolled back
    Aborted,
}

/// Operations that can be performed in a transaction
#[derive(Debug, Clone)]
pub enum Operation {
    /// Add a new chunk to the codebook
    AddChunk { chunk_id: ChunkId, data: Vec<u8> },
    /// Update an existing chunk
    UpdateChunk { chunk_id: ChunkId, data: Vec<u8> },
    /// Remove a chunk
    RemoveChunk { chunk_id: ChunkId },
    /// Add a new file to the manifest
    AddFile { path: String, chunks: Vec<ChunkId> },
    /// Update an existing file
    UpdateFile { path: String, chunks: Vec<ChunkId> },
    /// Remove a file
    RemoveFile { path: String },
    /// Bundle chunks into root
    BundleRoot { chunk_ids: Vec<ChunkId> },
}

/// A transaction
#[derive(Debug, Clone)]
pub struct Transaction {
    /// Unique transaction ID
    pub id: u64,

    /// Engram version at transaction start
    pub engram_version: u64,

    /// Operations in this transaction
    pub operations: Vec<Operation>,

    /// When the transaction started
    pub timestamp: Instant,

    /// Current status
    pub status: TransactionStatus,
}

impl Transaction {
    /// Create a new pending transaction
    pub fn new(id: u64, engram_version: u64) -> Self {
        Self {
            id,
            engram_version,
            operations: Vec::new(),
            timestamp: Instant::now(),
            status: TransactionStatus::Pending,
        }
    }

    /// Add an operation to the transaction
    pub fn add_operation(&mut self, op: Operation) {
        if self.status == TransactionStatus::Pending {
            self.operations.push(op);
        }
    }

    /// Mark the transaction as committed
    pub fn commit(&mut self) {
        if self.status == TransactionStatus::Pending {
            self.status = TransactionStatus::Committed;
        }
    }

    /// Mark the transaction as aborted
    pub fn abort(&mut self) {
        if self.status == TransactionStatus::Pending {
            self.status = TransactionStatus::Aborted;
        }
    }

    /// Get the age of the transaction
    pub fn age(&self) -> std::time::Duration {
        Instant::now().duration_since(self.timestamp)
    }
}

/// Transaction manager
pub struct TransactionManager {
    next_id: AtomicU64,
}

impl TransactionManager {
    /// Create a new transaction manager
    pub fn new() -> Self {
        Self {
            next_id: AtomicU64::new(0),
        }
    }

    /// Begin a new transaction
    pub fn begin(&self, engram_version: u64) -> Transaction {
        let id = self.next_id.fetch_add(1, Ordering::AcqRel);
        Transaction::new(id, engram_version)
    }
}

impl Default for TransactionManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transaction_creation() {
        let tx = Transaction::new(1, 0);
        assert_eq!(tx.id, 1);
        assert_eq!(tx.engram_version, 0);
        assert_eq!(tx.status, TransactionStatus::Pending);
        assert!(tx.operations.is_empty());
    }

    #[test]
    fn test_transaction_operations() {
        let mut tx = Transaction::new(1, 0);

        tx.add_operation(Operation::AddChunk {
            chunk_id: 1,
            data: vec![1, 2, 3],
        });

        assert_eq!(tx.operations.len(), 1);
    }

    #[test]
    fn test_transaction_commit() {
        let mut tx = Transaction::new(1, 0);
        tx.commit();
        assert_eq!(tx.status, TransactionStatus::Committed);

        // Can't add operations after commit
        tx.add_operation(Operation::AddChunk {
            chunk_id: 1,
            data: vec![],
        });
        assert_eq!(tx.operations.len(), 0);
    }

    #[test]
    fn test_transaction_manager() {
        let manager = TransactionManager::new();

        let tx1 = manager.begin(0);
        let tx2 = manager.begin(1);

        assert_eq!(tx1.id, 0);
        assert_eq!(tx2.id, 1);
    }
}
