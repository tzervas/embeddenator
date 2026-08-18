//! FFI (Foreign Function Interface) bindings for C/C++ integration.
//!
//! This module provides C-compatible bindings for embeddenator core types.
//! All functions are marked `unsafe` and require careful memory management.
//!
//! ## Safety Considerations
//!
//! - All pointers must be valid and properly aligned
//! - String pointers must be null-terminated UTF-8
//! - Caller is responsible for freeing memory allocated by FFI functions
//! - No Rust objects should be accessed after being freed
//!
//! ## Example (C)
//!
//! ```c
//! #include "embeddenator_interop.h"
//!
//! // Create a vector
//! SparseVecHandle* vec = sparse_vec_new();
//!
//! // Use the vector...
//!
//! // Free the vector
//! sparse_vec_free(vec);
//! ```

use embeddenator_vsa::{ReversibleVSAConfig, SparseVec};
use std::ffi::CStr;
use std::os::raw::c_char;
use std::ptr;
use std::slice;

// ============================================================================
// Opaque Handle Types
// ============================================================================

/// Opaque handle to a SparseVec
#[repr(C)]
pub struct SparseVecHandle {
    _private: [u8; 0],
}

/// Opaque handle to a ReversibleVSAConfig
#[repr(C)]
pub struct VSAConfigHandle {
    _private: [u8; 0],
}

/// Result buffer for returning data to C
#[repr(C)]
pub struct ByteBuffer {
    pub data: *mut u8,
    pub len: usize,
    pub capacity: usize,
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Convert a Rust object to an opaque pointer
unsafe fn to_handle<T>(obj: T) -> *mut T {
    Box::into_raw(Box::new(obj))
}

/// Convert an opaque pointer back to a Rust object
unsafe fn from_handle<T>(handle: *mut T) -> Box<T> {
    assert!(!handle.is_null(), "FFI: null handle");
    Box::from_raw(handle)
}

/// Borrow an object from a handle without taking ownership
unsafe fn borrow_handle<T>(handle: *const T) -> &'static T {
    assert!(!handle.is_null(), "FFI: null handle");
    &*handle
}

/// Borrow an object mutably from a handle without taking ownership
#[allow(dead_code)]
unsafe fn borrow_handle_mut<T>(handle: *mut T) -> &'static mut T {
    assert!(!handle.is_null(), "FFI: null handle");
    &mut *handle
}

// ============================================================================
// SparseVec FFI
// ============================================================================

/// Create a new SparseVec
///
/// # Safety
/// Must call `sparse_vec_free` to deallocate
#[no_mangle]
pub unsafe extern "C" fn sparse_vec_new() -> *mut SparseVecHandle {
    let vec = SparseVec::new();
    to_handle(vec) as *mut SparseVecHandle
}

/// Free a SparseVec
///
/// # Safety
/// - `handle` must be a valid pointer from `sparse_vec_new`
/// - Must not use `handle` after calling this function
#[no_mangle]
pub unsafe extern "C" fn sparse_vec_free(handle: *mut SparseVecHandle) {
    if !handle.is_null() {
        let _ = from_handle(handle as *mut SparseVec);
    }
}

/// Bundle two SparseVecs
///
/// # Safety
/// - All handles must be valid
/// - Returns a new handle that must be freed with `sparse_vec_free`
#[no_mangle]
pub unsafe extern "C" fn sparse_vec_bundle(
    a: *const SparseVecHandle,
    b: *const SparseVecHandle,
) -> *mut SparseVecHandle {
    let a_vec = borrow_handle(a as *const SparseVec);
    let b_vec = borrow_handle(b as *const SparseVec);
    let result = a_vec.bundle(b_vec);
    to_handle(result) as *mut SparseVecHandle
}

/// Bind two SparseVecs
///
/// # Safety
/// - All handles must be valid
/// - Returns a new handle that must be freed with `sparse_vec_free`
#[no_mangle]
pub unsafe extern "C" fn sparse_vec_bind(
    a: *const SparseVecHandle,
    b: *const SparseVecHandle,
) -> *mut SparseVecHandle {
    let a_vec = borrow_handle(a as *const SparseVec);
    let b_vec = borrow_handle(b as *const SparseVec);
    let result = a_vec.bind(b_vec);
    to_handle(result) as *mut SparseVecHandle
}

/// Compute cosine similarity between two SparseVecs
///
/// # Safety
/// All handles must be valid
#[no_mangle]
pub unsafe extern "C" fn sparse_vec_cosine(
    a: *const SparseVecHandle,
    b: *const SparseVecHandle,
) -> f64 {
    let a_vec = borrow_handle(a as *const SparseVec);
    let b_vec = borrow_handle(b as *const SparseVec);
    a_vec.cosine(b_vec)
}

/// Serialize a SparseVec to JSON
///
/// # Safety
/// - `handle` must be valid
/// - Returns a ByteBuffer that must be freed with `byte_buffer_free`
#[no_mangle]
pub unsafe extern "C" fn sparse_vec_to_json(handle: *const SparseVecHandle) -> ByteBuffer {
    let vec = borrow_handle(handle as *const SparseVec);
    match serde_json::to_vec(vec) {
        Ok(mut bytes) => {
            let len = bytes.len();
            let capacity = bytes.capacity();
            let data = bytes.as_mut_ptr();
            std::mem::forget(bytes);
            ByteBuffer {
                data,
                len,
                capacity,
            }
        }
        Err(_) => ByteBuffer {
            data: ptr::null_mut(),
            len: 0,
            capacity: 0,
        },
    }
}

/// Deserialize a SparseVec from JSON
///
/// # Safety
/// - `data` must point to valid JSON bytes
/// - `len` must be the correct length
/// - Returns a handle that must be freed with `sparse_vec_free`
#[no_mangle]
pub unsafe extern "C" fn sparse_vec_from_json(data: *const u8, len: usize) -> *mut SparseVecHandle {
    if data.is_null() {
        return ptr::null_mut();
    }
    let bytes = slice::from_raw_parts(data, len);
    match serde_json::from_slice::<SparseVec>(bytes) {
        Ok(vec) => to_handle(vec) as *mut SparseVecHandle,
        Err(_) => ptr::null_mut(),
    }
}

// ============================================================================
// VSAConfig FFI
// ============================================================================

/// Create a new default ReversibleVSAConfig
///
/// # Safety
/// Must call `vsa_config_free` to deallocate
#[no_mangle]
pub unsafe extern "C" fn vsa_config_new() -> *mut VSAConfigHandle {
    let config = ReversibleVSAConfig::default();
    to_handle(config) as *mut VSAConfigHandle
}

/// Create a new ReversibleVSAConfig with custom parameters
///
/// # Safety
/// Must call `vsa_config_free` to deallocate
#[no_mangle]
pub unsafe extern "C" fn vsa_config_new_custom(
    block_size: usize,
    max_path_depth: usize,
    base_shift: usize,
    target_sparsity: usize,
) -> *mut VSAConfigHandle {
    let config = ReversibleVSAConfig {
        block_size,
        max_path_depth,
        base_shift,
        target_sparsity,
    };
    to_handle(config) as *mut VSAConfigHandle
}

/// Free a VSAConfig
///
/// # Safety
/// - `handle` must be a valid pointer from `vsa_config_new*`
/// - Must not use `handle` after calling this function
#[no_mangle]
pub unsafe extern "C" fn vsa_config_free(handle: *mut VSAConfigHandle) {
    if !handle.is_null() {
        let _ = from_handle(handle as *mut ReversibleVSAConfig);
    }
}

/// Encode data into a SparseVec
///
/// # Safety
/// - All handles must be valid
/// - `data` must point to valid bytes
/// - `len` must be correct
/// - `path` may be null or must be null-terminated UTF-8
/// - Returns a handle that must be freed with `sparse_vec_free`
#[no_mangle]
pub unsafe extern "C" fn vsa_encode_data(
    config: *const VSAConfigHandle,
    data: *const u8,
    len: usize,
    path: *const c_char,
) -> *mut SparseVecHandle {
    let config_ref = borrow_handle(config as *const ReversibleVSAConfig);
    let bytes = slice::from_raw_parts(data, len);

    let path_str = if path.is_null() {
        None
    } else {
        CStr::from_ptr(path).to_str().ok()
    };

    let vec = SparseVec::encode_data(bytes, config_ref, path_str);
    to_handle(vec) as *mut SparseVecHandle
}

/// Decode a SparseVec back to data
///
/// # Safety
/// - All handles must be valid
/// - `path` may be null or must be null-terminated UTF-8
/// - Returns a ByteBuffer that must be freed with `byte_buffer_free`
#[no_mangle]
pub unsafe extern "C" fn vsa_decode_data(
    config: *const VSAConfigHandle,
    vec: *const SparseVecHandle,
    path: *const c_char,
    expected_size: usize,
) -> ByteBuffer {
    let config_ref = borrow_handle(config as *const ReversibleVSAConfig);
    let vec_ref = borrow_handle(vec as *const SparseVec);

    let path_str = if path.is_null() {
        None
    } else {
        CStr::from_ptr(path).to_str().ok()
    };

    let mut decoded = vec_ref.decode_data(config_ref, path_str, expected_size);
    let len = decoded.len();
    let capacity = decoded.capacity();
    let data = decoded.as_mut_ptr();
    std::mem::forget(decoded);

    ByteBuffer {
        data,
        len,
        capacity,
    }
}

// ============================================================================
// ByteBuffer Management
// ============================================================================

/// Free a ByteBuffer returned by FFI functions
///
/// # Safety
/// - `buffer` must be a valid ByteBuffer returned by this library
/// - Must not use `buffer` after calling this function
#[no_mangle]
pub unsafe extern "C" fn byte_buffer_free(buffer: ByteBuffer) {
    if !buffer.data.is_null() {
        let _ = Vec::from_raw_parts(buffer.data, buffer.len, buffer.capacity);
    }
}

// ============================================================================
// Error Handling
// ============================================================================

/// Get the last error message (if any)
///
/// # Safety
/// - Returns a pointer to a static string (do not free)
/// - Returns null if no error occurred
#[no_mangle]
pub unsafe extern "C" fn embeddenator_last_error() -> *const c_char {
    // Thread-local error storage could be added here
    ptr::null()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sparse_vec_create_free() {
        unsafe {
            let handle = sparse_vec_new();
            assert!(!handle.is_null());
            sparse_vec_free(handle);
        }
    }

    #[test]
    fn test_sparse_vec_operations() {
        unsafe {
            let a = sparse_vec_new();
            let b = sparse_vec_new();

            let bundled = sparse_vec_bundle(a, b);
            assert!(!bundled.is_null());

            let bound = sparse_vec_bind(a, b);
            assert!(!bound.is_null());

            let cosine = sparse_vec_cosine(a, b);
            assert!(cosine.is_finite());

            sparse_vec_free(bundled);
            sparse_vec_free(bound);
            sparse_vec_free(a);
            sparse_vec_free(b);
        }
    }

    #[test]
    fn test_sparse_vec_json_roundtrip() {
        unsafe {
            let vec = sparse_vec_new();

            let buffer = sparse_vec_to_json(vec);
            assert!(!buffer.data.is_null());
            assert!(buffer.len > 0);

            let decoded = sparse_vec_from_json(buffer.data, buffer.len);
            assert!(!decoded.is_null());

            byte_buffer_free(buffer);
            sparse_vec_free(vec);
            sparse_vec_free(decoded);
        }
    }

    #[test]
    fn test_vsa_config() {
        unsafe {
            let config = vsa_config_new();
            assert!(!config.is_null());
            vsa_config_free(config);

            let custom = vsa_config_new_custom(256, 10, 1000, 200);
            assert!(!custom.is_null());
            vsa_config_free(custom);
        }
    }

    #[test]
    fn test_encode_decode() {
        unsafe {
            let config = vsa_config_new();
            let data = b"Hello, FFI!";

            let vec = vsa_encode_data(config, data.as_ptr(), data.len(), ptr::null());
            assert!(!vec.is_null());

            let decoded = vsa_decode_data(config, vec, ptr::null(), data.len());
            assert!(!decoded.data.is_null());
            assert_eq!(decoded.len, data.len());

            byte_buffer_free(decoded);
            sparse_vec_free(vec);
            vsa_config_free(config);
        }
    }
}
