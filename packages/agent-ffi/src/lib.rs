//! C ABI consumed by the Flutter desktop app through `dart:ffi`.
//!
//! Rules for this crate:
//!
//! - Every string returned to Dart is allocated by this crate and released with
//!   [`ll10_free_string`].
//! - Data crosses the boundary as UTF-8 JSON, never as a Rust struct layout.
//! - All `unsafe` code lives here, behind a small and audited surface.
//!
//! ABI version history:
//!
//! - `1`: `ll10_abi_version`, `ll10_diagnostics_json`, `ll10_free_string`.

use std::ffi::{c_char, CString};

/// Version of the C ABI, incremented on any breaking change.
pub const ABI_VERSION: u32 = 1;

/// Returns the ABI version so the Dart side can refuse an incompatible library.
#[no_mangle]
pub extern "C" fn ll10_abi_version() -> u32 {
    ABI_VERSION
}

/// Returns diagnostics as a null-terminated UTF-8 JSON string.
///
/// The caller owns the returned pointer and must release it with
/// [`ll10_free_string`]. Returns a null pointer when serialization fails.
#[no_mangle]
pub extern "C" fn ll10_diagnostics_json() -> *mut c_char {
    match agent::diagnostics().to_json() {
        Ok(json) => into_c_string(json),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Releases a string previously returned by this crate.
///
/// Passing a null pointer is allowed and does nothing. Passing a pointer that
/// was not produced by this crate is undefined behavior.
///
/// # Safety
///
/// The pointer must come from this crate and must not be freed twice.
#[no_mangle]
pub unsafe extern "C" fn ll10_free_string(value: *mut c_char) {
    if value.is_null() {
        return;
    }

    drop(unsafe { CString::from_raw(value) });
}

/// Converts an owned Rust string into a pointer handed to Dart.
fn into_c_string(value: String) -> *mut c_char {
    match CString::new(value) {
        Ok(text) => text.into_raw(),
        Err(_) => std::ptr::null_mut(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CStr;

    #[test]
    fn abi_version_is_reported() {
        assert_eq!(ll10_abi_version(), ABI_VERSION);
    }

    #[test]
    fn diagnostics_json_is_valid_utf8_json() {
        let pointer = ll10_diagnostics_json();
        assert!(!pointer.is_null(), "diagnostics must serialize");

        let json = unsafe { CStr::from_ptr(pointer) }
            .to_str()
            .expect("diagnostics are valid UTF-8");

        let parsed: serde_json::Value = serde_json::from_str(json).expect("diagnostics are JSON");
        assert!(parsed.get("agent_version").is_some());
        assert!(parsed.get("architecture").is_some());

        unsafe { ll10_free_string(pointer) };
    }

    #[test]
    fn freeing_a_null_pointer_is_safe() {
        unsafe { ll10_free_string(std::ptr::null_mut()) };
    }

    #[test]
    fn repeated_calls_return_independent_strings() {
        let first = ll10_diagnostics_json();
        let second = ll10_diagnostics_json();

        assert!(!first.is_null());
        assert!(!second.is_null());
        assert_ne!(first, second);

        unsafe {
            ll10_free_string(first);
            ll10_free_string(second);
        }
    }
}
