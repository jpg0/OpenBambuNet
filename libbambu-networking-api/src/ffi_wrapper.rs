/// Safe Rust wrappers around C FFI for ABI compatibility testing
/// 
/// This module provides typesafe wrappers around the C-compatible FFI,
/// handling memory safety and error checking.

use std::ffi::CStr;
use std::os::raw::c_char;

// Raw C FFI declarations
extern "C" {
    fn bambu_network_get_version_c() -> *const c_char;
    fn bambu_network_init_c();
    fn bambu_network_connect_c(dev_id: *const c_char, ip: *const c_char, password: *const c_char, ssl: bool) -> i32;
    fn bambu_network_disconnect_c(dev_id: *const c_char) -> i32;
    fn bambu_network_send_c(dev_id: *const c_char, data: *const c_char) -> i32;
}

/// Error type for FFI operations
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FfiError {
    NullPointer,
    InvalidUtf8,
    CStringError,
}

impl std::fmt::Display for FfiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FfiError::NullPointer => write!(f, "FFI function returned null pointer"),
            FfiError::InvalidUtf8 => write!(f, "FFI function returned invalid UTF-8"),
            FfiError::CStringError => write!(f, "Failed to create CString"),
        }
    }
}

impl std::error::Error for FfiError {}

/// Get the library version string
pub fn get_version() -> Result<String, FfiError> {
    unsafe {
        let ptr = bambu_network_get_version_c();
        if ptr.is_null() {
            return Err(FfiError::NullPointer);
        }
        
        CStr::from_ptr(ptr)
            .to_str()
            .map(|s| s.to_string())
            .map_err(|_| FfiError::InvalidUtf8)
    }
}

/// Initialize the agent
pub fn init() {
    unsafe {
        bambu_network_init_c();
    }
}

/// Connect to a printer
pub fn connect(dev_id: &str, ip: &str, password: &str, ssl: bool) -> Result<i32, FfiError> {
    let c_dev_id = std::ffi::CString::new(dev_id).map_err(|_| FfiError::CStringError)?;
    let c_ip = std::ffi::CString::new(ip).map_err(|_| FfiError::CStringError)?;
    let c_password = std::ffi::CString::new(password).map_err(|_| FfiError::CStringError)?;
    
    unsafe {
        Ok(bambu_network_connect_c(c_dev_id.as_ptr(), c_ip.as_ptr(), c_password.as_ptr(), ssl))
    }
}

/// Disconnect from a printer
pub fn disconnect(dev_id: &str) -> Result<i32, FfiError> {
    let c_dev_id = std::ffi::CString::new(dev_id).map_err(|_| FfiError::CStringError)?;
    unsafe {
        Ok(bambu_network_disconnect_c(c_dev_id.as_ptr()))
    }
}

/// Send a message to a printer
pub fn send(dev_id: &str, data: &str) -> Result<i32, FfiError> {
    let c_dev_id = std::ffi::CString::new(dev_id).map_err(|_| FfiError::CStringError)?;
    let c_data = std::ffi::CString::new(data).map_err(|_| FfiError::CStringError)?;
    unsafe {
        Ok(bambu_network_send_c(c_dev_id.as_ptr(), c_data.as_ptr()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_version_not_null() {
        let version = get_version();
        assert!(version.is_ok(), "get_version should not return error");
    }

    #[test]
    fn test_get_version_not_empty() {
        let version = get_version().expect("get_version failed");
        assert!(!version.is_empty(), "version should not be empty");
    }

    #[test]
    fn test_get_version_valid_format() {
        let version = get_version().expect("get_version failed");
        // Version should contain digits and periods (e.g., "01.07.07.89")
        assert!(version.chars().any(|c| c.is_ascii_digit()), 
                "version should contain digits: {}", version);
    }
}
