//! Database-like functionality tests for VersionedEmbrFS
//!
//! Tests structured/unstructured data storage, transactional operations,
//! and database-like patterns with high concurrency.

use embeddenator_fs::{EmbrFSError, VersionedEmbrFS};
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering as AtomicOrdering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::SystemTime;

// === Structured Data Types ===

#[derive(Debug, Clone, PartialEq)]
struct User {
    id: u64,
    username: String,
    email: String,
    created_at: u64,
}

impl User {
    fn to_bytes(&self) -> Vec<u8> {
        // Simple serialization (in production use serde)
        format!(
            "{}|{}|{}|{}",
            self.id, self.username, self.email, self.created_at
        )
        .into_bytes()
    }

    fn from_bytes(data: &[u8]) -> Self {
        let s = String::from_utf8_lossy(data);
        let parts: Vec<&str> = s.split('|').collect();
        User {
            id: parts[0].parse().unwrap(),
            username: parts[1].to_string(),
            email: parts[2].to_string(),
            created_at: parts[3].parse().unwrap(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
struct Transaction {
    id: u64,
    user_id: u64,
    amount: f64,
    timestamp: u64,
    status: String,
}

impl Transaction {
    fn to_bytes(&self) -> Vec<u8> {
        format!(
            "{}|{}|{}|{}|{}",
            self.id, self.user_id, self.amount, self.timestamp, self.status
        )
        .into_bytes()
    }

    fn from_bytes(data: &[u8]) -> Self {
        let s = String::from_utf8_lossy(data);
        let parts: Vec<&str> = s.split('|').collect();
        Transaction {
            id: parts[0].parse().unwrap(),
            user_id: parts[1].parse().unwrap(),
            amount: parts[2].parse().unwrap(),
            timestamp: parts[3].parse().unwrap(),
            status: parts[4].to_string(),
        }
    }
}

// === Basic CRUD Operations ===

#[test]
fn test_crud_structured_data() {
    let fs = VersionedEmbrFS::new();

    // CREATE
    let user = User {
        id: 1,
        username: "alice".to_string(),
        email: "alice@example.com".to_string(),
        created_at: 1234567890,
    };

    let version = fs
        .write_file("users/1.dat", &user.to_bytes(), None)
        .unwrap();

    // READ
    let (data, read_version) = fs.read_file("users/1.dat").unwrap();
    let read_user = User::from_bytes(&data);
    assert_eq!(read_user, user);
    assert_eq!(read_version, version);

    // UPDATE
    let mut updated_user = user.clone();
    updated_user.email = "alice_new@example.com".to_string();

    let new_version = fs
        .write_file("users/1.dat", &updated_user.to_bytes(), Some(version))
        .unwrap();
    assert!(new_version > version);

    // Verify update
    let (data, _) = fs.read_file("users/1.dat").unwrap();
    let final_user = User::from_bytes(&data);
    assert_eq!(final_user.email, "alice_new@example.com");

    // DELETE
    fs.delete_file("users/1.dat", new_version).unwrap();
    assert!(!fs.exists("users/1.dat"));
}

#[test]
fn test_multiple_records() {
    let fs = VersionedEmbrFS::new();

    // Create 1000 user records
    for i in 0..1000 {
        let user = User {
            id: i,
            username: format!("user_{}", i),
            email: format!("user_{}@example.com", i),
            created_at: 1234567890 + i,
        };

        fs.write_file(&format!("users/{}.dat", i), &user.to_bytes(), None)
            .unwrap();
    }

    // Verify all records
    assert_eq!(fs.list_files().len(), 1000);

    // Read random records
    for i in [0, 42, 100, 500, 999] {
        let (data, _) = fs.read_file(&format!("users/{}.dat", i)).unwrap();
        let user = User::from_bytes(&data);
        assert_eq!(user.id, i);
    }
}

// === Transaction Patterns ===

#[test]
fn test_atomic_transaction_pattern() {
    let fs = Arc::new(VersionedEmbrFS::new());

    // Simulate atomic transaction: transfer money between accounts
    let account_a_path = "accounts/a.dat";
    let account_b_path = "accounts/b.dat";

    // Initialize accounts
    fs.write_file(account_a_path, b"1000.00", None).unwrap();
    fs.write_file(account_b_path, b"500.00", None).unwrap();

    // Transfer $100 from A to B
    loop {
        // Read both accounts
        let (data_a, version_a) = fs.read_file(account_a_path).unwrap();
        let (data_b, version_b) = fs.read_file(account_b_path).unwrap();

        let balance_a: f64 = String::from_utf8_lossy(&data_a).parse().unwrap();
        let balance_b: f64 = String::from_utf8_lossy(&data_b).parse().unwrap();

        // Calculate new balances
        let new_balance_a = balance_a - 100.0;
        let new_balance_b = balance_b + 100.0;

        // Try to update both (both must succeed or rollback)
        let result_a = fs.write_file(
            account_a_path,
            format!("{:.2}", new_balance_a).as_bytes(),
            Some(version_a),
        );

        if result_a.is_ok() {
            let result_b = fs.write_file(
                account_b_path,
                format!("{:.2}", new_balance_b).as_bytes(),
                Some(version_b),
            );

            match result_b {
                Ok(_) => break, // Transaction complete
                Err(EmbrFSError::VersionMismatch { .. }) => {
                    // Rollback A and retry
                    let rollback_data = format!("{:.2}", balance_a).as_bytes().to_vec();
                    loop {
                        let (_, current_version) = fs.read_file(account_a_path).unwrap();
                        match fs.write_file(account_a_path, &rollback_data, Some(current_version)) {
                            Ok(_) => break,
                            Err(_) => continue,
                        }
                    }
                    continue; // Retry transaction
                }
                Err(e) => panic!("Unexpected error: {:?}", e),
            }
        }
    }

    // Verify final balances
    let (data_a, _) = fs.read_file(account_a_path).unwrap();
    let (data_b, _) = fs.read_file(account_b_path).unwrap();
    let final_a: f64 = String::from_utf8_lossy(&data_a).parse().unwrap();
    let final_b: f64 = String::from_utf8_lossy(&data_b).parse().unwrap();

    assert_eq!(final_a, 900.00);
    assert_eq!(final_b, 600.00);
}

#[test]
fn test_concurrent_transactions() {
    let fs = Arc::new(VersionedEmbrFS::new());

    // Initialize account
    fs.write_file("account.dat", b"0.00", None).unwrap();

    let mut handles = vec![];
    let success_count = Arc::new(AtomicUsize::new(0));

    // 50 threads each trying to deposit $10
    for thread_id in 0..50 {
        let fs_clone = Arc::clone(&fs);
        let success_count_clone = Arc::clone(&success_count);

        let handle = thread::spawn(move || {
            // Retry loop for optimistic locking
            loop {
                let (data, version) = fs_clone.read_file("account.dat").unwrap();
                let balance: f64 = String::from_utf8_lossy(&data).parse().unwrap();
                let new_balance = balance + 10.0;

                match fs_clone.write_file(
                    "account.dat",
                    format!("{:.2}", new_balance).as_bytes(),
                    Some(version),
                ) {
                    Ok(_) => {
                        success_count_clone.fetch_add(1, AtomicOrdering::AcqRel);
                        break;
                    }
                    Err(EmbrFSError::VersionMismatch { .. }) => continue,
                    Err(e) => panic!("Unexpected error in thread {}: {:?}", thread_id, e),
                }
            }
        });
        handles.push(handle);
    }

    // Wait for all transactions
    for handle in handles {
        handle.join().unwrap();
    }

    // Verify final balance (should be 50 * $10 = $500)
    let (data, _) = fs.read_file("account.dat").unwrap();
    let final_balance: f64 = String::from_utf8_lossy(&data).parse().unwrap();
    assert_eq!(final_balance, 500.00);
    assert_eq!(success_count.load(AtomicOrdering::Acquire), 50);
}

#[test]
fn test_batch_transaction_pattern() {
    let fs = VersionedEmbrFS::new();

    // Batch insert transactions
    let transactions: Vec<Transaction> = (0..1000)
        .map(|i| Transaction {
            id: i,
            user_id: i % 100,
            amount: (i as f64) * 10.5,
            timestamp: 1234567890 + i,
            status: "completed".to_string(),
        })
        .collect();

    // Write all transactions
    for tx in &transactions {
        fs.write_file(&format!("transactions/{}.dat", tx.id), &tx.to_bytes(), None)
            .unwrap();
    }

    // Verify count
    assert_eq!(fs.list_files().len(), 1000);

    // Query by user_id (simulated index scan)
    let user_50_txs: Vec<Transaction> = (0..1000)
        .filter(|i| i % 100 == 50)
        .map(|i| {
            let (data, _) = fs.read_file(&format!("transactions/{}.dat", i)).unwrap();
            Transaction::from_bytes(&data)
        })
        .collect();

    assert_eq!(user_50_txs.len(), 10);
    for tx in user_50_txs {
        assert_eq!(tx.user_id, 50);
    }
}

// === Unstructured Data ===

#[test]
fn test_json_like_documents() {
    let fs = VersionedEmbrFS::new();

    // Simulate JSON document storage
    let doc1 = br#"{"id": 1, "name": "Alice", "tags": ["admin", "active"], "score": 95.5}"#;
    let doc2 = br#"{"id": 2, "name": "Bob", "tags": ["user"], "score": 87.2, "extra": "data"}"#;

    fs.write_file("docs/1.json", doc1, None).unwrap();
    fs.write_file("docs/2.json", doc2, None).unwrap();

    // Retrieve and verify
    let (data, _) = fs.read_file("docs/1.json").unwrap();
    assert_eq!(&data[..], doc1);
}

#[test]
fn test_key_value_store() {
    let fs = VersionedEmbrFS::new();

    // Use filesystem as key-value store
    let kv_pairs = vec![
        ("config/database_url", b"postgresql://localhost/db".as_ref()),
        ("config/api_key", b"sk_test_123456789".as_ref()),
        ("config/max_connections", b"100".as_ref()),
        ("state/last_sync", b"1234567890".as_ref()),
        ("state/user_count", b"42".as_ref()),
    ];

    // Put all keys
    for (key, value) in &kv_pairs {
        fs.write_file(key, value, None).unwrap();
    }

    // Get all keys
    for (key, expected_value) in &kv_pairs {
        let (value, _) = fs.read_file(key).unwrap();
        assert_eq!(&value[..], *expected_value);
    }

    // Update a key
    let (_, version) = fs.read_file("state/user_count").unwrap();
    fs.write_file("state/user_count", b"43", Some(version))
        .unwrap();

    let (updated, _) = fs.read_file("state/user_count").unwrap();
    assert_eq!(&updated[..], b"43");
}

#[test]
fn test_time_series_data() {
    let fs = VersionedEmbrFS::new();

    // Simulate time series metrics
    let start_time = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    for i in 0..100 {
        let timestamp = start_time + i;
        let metric = format!("{}|cpu|{:.2}", timestamp, 50.0 + (i as f64) * 0.5);
        fs.write_file(
            &format!("metrics/{}.dat", timestamp),
            metric.as_bytes(),
            None,
        )
        .unwrap();
    }

    // Read back time series
    let files = fs.list_files();
    assert_eq!(files.len(), 100);
}

// === Concurrent Database Operations ===

#[test]
fn test_concurrent_read_write_mix() {
    let fs = Arc::new(VersionedEmbrFS::new());

    // Initialize 100 records
    for i in 0..100 {
        let user = User {
            id: i,
            username: format!("user_{}", i),
            email: format!("user_{}@example.com", i),
            created_at: 1234567890,
        };
        fs.write_file(&format!("users/{}.dat", i), &user.to_bytes(), None)
            .unwrap();
    }

    let mut handles = vec![];

    // 20 reader threads
    for reader_id in 0..20 {
        let fs_clone = Arc::clone(&fs);
        let handle = thread::spawn(move || {
            for iteration in 0..50 {
                let id = ((reader_id + iteration) % 100) as u64;
                let (data, _) = fs_clone.read_file(&format!("users/{}.dat", id)).unwrap();
                let user = User::from_bytes(&data);
                assert_eq!(user.id, id);
            }
        });
        handles.push(handle);
    }

    // 10 writer threads (updating records)
    for writer_id in 0..10 {
        let fs_clone = Arc::clone(&fs);
        let handle = thread::spawn(move || {
            for _ in 0..20 {
                let id = (writer_id * 10) as u64;
                loop {
                    let (data, version) = fs_clone.read_file(&format!("users/{}.dat", id)).unwrap();
                    let mut user = User::from_bytes(&data);
                    user.email = format!("updated_{}@example.com", id);

                    match fs_clone.write_file(
                        &format!("users/{}.dat", id),
                        &user.to_bytes(),
                        Some(version),
                    ) {
                        Ok(_) => break,
                        Err(EmbrFSError::VersionMismatch { .. }) => continue,
                        Err(e) => panic!("Unexpected error: {:?}", e),
                    }
                }
            }
        });
        handles.push(handle);
    }

    // Wait for all operations
    for handle in handles {
        handle.join().unwrap();
    }
}

#[test]
fn test_pessimistic_locking_pattern() {
    let fs = Arc::new(VersionedEmbrFS::new());
    let lock = Arc::new(Mutex::new(()));

    // Initialize shared resource
    fs.write_file("counter.dat", b"0", None).unwrap();

    let mut handles = vec![];

    // 20 threads with explicit locking
    for _ in 0..20 {
        let fs_clone = Arc::clone(&fs);
        let lock_clone = Arc::clone(&lock);

        let handle = thread::spawn(move || {
            for _ in 0..10 {
                // Acquire external lock
                let _guard = lock_clone.lock().unwrap();

                // Now safe to read-modify-write
                let (data, version) = fs_clone.read_file("counter.dat").unwrap();
                let count: u64 = String::from_utf8_lossy(&data).parse().unwrap();
                let new_count = count + 1;

                fs_clone
                    .write_file(
                        "counter.dat",
                        new_count.to_string().as_bytes(),
                        Some(version),
                    )
                    .unwrap();

                // Lock released when _guard drops
            }
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.join().unwrap();
    }

    // Verify final count
    let (data, _) = fs.read_file("counter.dat").unwrap();
    let final_count: u64 = String::from_utf8_lossy(&data).parse().unwrap();
    assert_eq!(final_count, 200);
}

#[test]
fn test_multi_version_consistency() {
    let fs = Arc::new(VersionedEmbrFS::new());

    // Create initial record
    let user = User {
        id: 1,
        username: "alice".to_string(),
        email: "alice@v1.com".to_string(),
        created_at: 1234567890,
    };
    fs.write_file("user.dat", &user.to_bytes(), None).unwrap();

    let versions = Arc::new(Mutex::new(Vec::new()));

    let mut handles = vec![];

    // 10 threads making sequential updates
    for i in 0..10 {
        let fs_clone = Arc::clone(&fs);
        let versions_clone = Arc::clone(&versions);

        let handle = thread::spawn(move || loop {
            let (data, version) = fs_clone.read_file("user.dat").unwrap();
            let mut user = User::from_bytes(&data);
            user.email = format!("alice@v{}.com", i);

            match fs_clone.write_file("user.dat", &user.to_bytes(), Some(version)) {
                Ok(new_version) => {
                    versions_clone.lock().unwrap().push(new_version);
                    break;
                }
                Err(EmbrFSError::VersionMismatch { .. }) => continue,
                Err(e) => panic!("Unexpected error: {:?}", e),
            }
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.join().unwrap();
    }

    // Verify version monotonicity
    let versions = versions.lock().unwrap();
    assert_eq!(versions.len(), 10);

    let mut sorted = versions.clone();
    sorted.sort();
    assert_eq!(
        *versions, sorted,
        "Versions should be monotonically increasing"
    );
}

#[test]
fn test_snapshot_isolation() {
    let fs = Arc::new(VersionedEmbrFS::new());

    // Create 10 records
    for i in 0..10 {
        fs.write_file(
            &format!("record_{}.dat", i),
            format!("value_{}", i).as_bytes(),
            None,
        )
        .unwrap();
    }

    // Take snapshot (read all versions)
    let snapshot: HashMap<String, u64> = (0..10)
        .map(|i| {
            let path = format!("record_{}.dat", i);
            let (_, version) = fs.read_file(&path).unwrap();
            (path, version)
        })
        .collect();

    // Now update all records
    for i in 0..10 {
        loop {
            let path = format!("record_{}.dat", i);
            let (_, version) = fs.read_file(&path).unwrap();
            match fs.write_file(&path, format!("updated_{}", i).as_bytes(), Some(version)) {
                Ok(_) => break,
                Err(_) => continue,
            }
        }
    }

    // Verify snapshot consistency - old versions should detect mismatches
    for (path, old_version) in snapshot {
        let result = fs.write_file(&path, b"test", Some(old_version));
        assert!(matches!(result, Err(EmbrFSError::VersionMismatch { .. })));
    }
}
