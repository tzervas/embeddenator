# RwLock Poisoning Recovery Strategy

## Overview

This document describes the lock poisoning recovery strategy implemented in `src/fs/fuse_shim.rs` to handle RwLock failures gracefully in long-running processes.

## Problem

RwLock poisoning occurs when a thread panics while holding a lock. By default, all subsequent attempts to acquire the lock will panic, causing a cascade failure across all threads trying to access the locked data.

For a filesystem like EngramFS, this creates two critical issues:
1. **Availability**: A single panic could render the entire filesystem inaccessible
2. **Data Loss**: Applications reading from the mounted filesystem would fail unexpectedly

## Solution Strategy

We implement a dual-strategy approach based on the operation type:

### 1. Write Operations (Mutating State)

**Strategy**: Propagate errors via `Result` types

**Rationale**: Write operations modify filesystem state. If a lock is poisoned during a write, we cannot guarantee data consistency. It's safer to fail the operation and return an error than to potentially corrupt the filesystem structure.

**Implementation**:
```rust
self.inodes.write()
    .map_err(|_| "Inodes lock poisoned")?
    .insert(ino, attr);
```

**Affected Functions**:
- `alloc_ino()` - Returns `Result<Ino, &'static str>`
- `add_file()` - Already returns `Result`, errors propagate up
- `ensure_directory()` - Already returns `Result`, errors propagate up
- `init_root()` - Uses `expect()` since initialization failure is unrecoverable

### 2. Read Operations (Non-Mutating Queries)

**Strategy**: Recover by accessing poisoned data via `into_inner()`

**Rationale**: Read operations don't modify state. Even if the lock is poisoned, the underlying data structure (HashMap) is likely still valid and readable. Continuing to serve read requests (with logging) is better than failing all filesystem operations.

**Implementation**:
```rust
let cache = self.file_cache.read().unwrap_or_else(|poisoned| {
    eprintln!("WARNING: file_cache lock poisoned in read_data, recovering...");
    poisoned.into_inner()
});
```

**Affected Functions**:
- `lookup_path()` - Recovers and continues
- `get_attr()` - Recovers and continues
- `read_data()` - Recovers and continues
- `read_dir()` - Recovers and continues
- `lookup_entry()` - Recovers and continues
- `get_parent()` - Recovers and continues (uses 2 read locks)
- `file_count()` - Recovers and continues
- `total_size()` - Recovers and continues

## Logging Strategy

All lock poisoning events are logged to stderr using `eprintln!()`:

```
WARNING: <lock_name> lock poisoned in <function_name>, recovering...
```

This provides visibility into lock poisoning events for:
- Debugging the root cause of panics
- Monitoring filesystem health
- Alerting on degraded service conditions

## Trade-offs

### Advantages
- **High Availability**: Filesystem continues serving read requests even with poisoned locks
- **Graceful Degradation**: Write operations fail cleanly rather than crashing
- **Visibility**: All poisoning events are logged
- **Data Safety**: Write operations won't corrupt filesystem state

### Disadvantages
- **Stale Data Risk**: If a write panic corrupted data, reads might see inconsistent state
- **Silent Degradation**: Read operations continue despite underlying problems
- **Limited Write Recovery**: Write operations cannot recover from poisoning

## Testing

Two test cases verify the recovery strategy:

1. **`test_lock_poisoning_recovery()`**
   - Verifies read operations work correctly
   - Tests concurrent access patterns
   - Ensures filesystem remains functional

2. **`test_write_lock_error_propagation()`**
   - Verifies write operations return proper errors
   - Ensures filesystem state remains consistent

## Monitoring Recommendations

In production deployments, monitor for:

1. **Stderr logs** containing "lock poisoned" warnings
2. **Increased error rates** on write operations
3. **Thread panic traces** that might cause lock poisoning

## Future Improvements

Potential enhancements to the recovery strategy:

1. **Structured Logging**: Replace `eprintln!()` with proper logging framework
2. **Metrics**: Expose lock poisoning counters via metrics/telemetry
3. **Lock Recreation**: For some scenarios, recreate the lock with recovered data
4. **Read Validation**: Add checksums to detect data corruption after recovery
5. **Alerting Integration**: Trigger alerts on lock poisoning events

## Changed Lines Summary

Fixed 27 RwLock operations in `src/fs/fuse_shim.rs`:

- **Lines 276-279**: `init_root()` - 4 write locks (added expect with context)
- **Line 284**: `alloc_ino()` - 1 write lock (error propagation)
- **Line 304**: `add_file()` - 1 read lock (recovery)
- **Lines 327-330**: `add_file()` - 4 write locks (error propagation)
- **Line 334**: `add_file()` - 1 write lock (error propagation)
- **Line 356**: `ensure_directory()` - 1 read lock (recovery)
- **Lines 376-379**: `ensure_directory()` - 4 write locks (error propagation)
- **Line 383**: `ensure_directory()` - 1 write lock (error propagation)
- **Line 393**: `ensure_directory()` - 1 write lock (error propagation)
- **Line 403**: `lookup_path()` - 1 read lock (recovery)
- **Line 408**: `get_attr()` - 1 read lock (recovery)
- **Line 413**: `read_data()` - 1 read lock (recovery)
- **Line 428**: `read_dir()` - 1 read lock (recovery)
- **Line 433**: `lookup_entry()` - 1 read lock (recovery)
- **Lines 444, 448**: `get_parent()` - 2 read locks (recovery)
- **Line 454**: `file_count()` - 1 read lock (recovery)
- **Line 459**: `total_size()` - 1 read lock (recovery)

## References

- Rust RwLock documentation: https://doc.rust-lang.org/std/sync/struct.RwLock.html
- Lock poisoning: https://doc.rust-lang.org/std/sync/struct.PoisonError.html
- QA Audit: `QA_AUDIT_1.0.0_READINESS.md` - Priority 2 issues
