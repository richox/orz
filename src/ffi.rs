use super::lz::LZCfg;
use super::{decode, encode, Stat};
use libc::size_t;
use std::error;
use std::ffi::CStr;
use std::fs;
use std::io;
use std::os::raw::c_char;
use std::ptr;
use std::slice;

// C FFI forwarders
fn handle_option(result: io::Result<Stat>) -> *const Stat {
    match result {
        Ok(r) => {
            let raw = Box::new(r);
            Box::into_raw(raw)
        }
        // TODO: find a way to indicate errors
        Err(_) => ptr::null(),
    }
}

/// Encode a buffer to a buffer.
/// No pointers may be NULL at this point.
///
/// Returns a pointer to a Stat structure if successful; otherwise
/// returns NULL.
///
/// # Safety
/// If your input sizes are bad, expect me to run out of bounds.
#[no_mangle]
pub unsafe extern "C" fn orz_encode_buf(
    source: *const u8,
    nsource: size_t,
    target: *mut u8,
    ntarget: size_t,
    cfg: *const LZCfg,
) -> *const Stat {
    if source.is_null() || target.is_null() || cfg.is_null() {
        ptr::null()
    } else {
        let result = encode(
            &mut slice::from_raw_parts(source, nsource),
            &mut slice::from_raw_parts_mut(target, ntarget),
            &*cfg,
        );
        handle_option(result)
    }
}

/// Decode a buffer from a buffer.
/// No pointers may be NULL at this point.
///
/// Returns a pointer to a Stat structure if successful; otherwise
/// returns NULL.
///
/// # Safety
/// If your input sizes are bad, expect me to run out of bounds.
#[no_mangle]
pub unsafe extern "C" fn orz_decode_buf(
    source: *const u8,
    nsource: size_t,
    target: *mut u8,
    ntarget: size_t,
) -> *const Stat {
    if source.is_null() || target.is_null() {
        ptr::null()
    } else {
        let result = decode(
            &mut slice::from_raw_parts(source, nsource),
            &mut slice::from_raw_parts_mut(target, ntarget),
        );
        handle_option(result)
    }
}

unsafe fn dofile<'a>(
    file: *const c_char,
    fun: fn(&'a str) -> io::Result<fs::File>,
) -> Result<fs::File, Box<dyn error::Error>> {
    let fc = CStr::from_ptr(file);
    let fu = fc.to_str()?;
    let fs = fun(fu);
    match fs {
        Ok(r) => Ok(r),
        Err(e) => Err(Box::new(e)),
    }
}

unsafe fn openfile(f: *const c_char) -> Result<fs::File, Box<dyn error::Error>> {
    dofile(f, fs::File::open)
}

unsafe fn createfile(f: *const c_char) -> Result<fs::File, Box<dyn error::Error>> {
    dofile(f, fs::File::create)
}

/// Encode a file path to a file path.
///
/// # Safety
/// Don't put garbage in LZCfg. Make sure these are UTF-8 encoded C strings.
#[no_mangle]
pub unsafe extern "C" fn orz_encode_path(
    source: *const c_char,
    target: *const c_char,
    cfg: *const LZCfg,
) -> *const Stat {
    if source.is_null() || target.is_null() || cfg.is_null() {
        ptr::null()
    } else {
        let files = (openfile(source), createfile(target));
        match files {
            (Ok(mut s), Ok(mut t)) => {
                let result = encode(&mut s, &mut t, &*cfg);
                handle_option(result)
            }
            _ => ptr::null(),
        }
    }
}

/// Decode a file path to a file path.
///
/// # Safety
/// Don't put garbage in LZCfg. Make sure these are UTF-8 encoded C strings.
#[no_mangle]
pub unsafe extern "C" fn orz_decode_path(
    source: *const c_char,
    target: *const c_char,
) -> *const Stat {
    if source.is_null() || target.is_null() {
        ptr::null()
    } else {
        let files = (openfile(source), createfile(target));
        match files {
            (Ok(mut s), Ok(mut t)) => {
                let result = decode(&mut s, &mut t);
                handle_option(result)
            }
            _ => ptr::null(),
        }
    }
}

/// Let the rust system take the pointer back.
///
/// # Safety
/// May cause a double free. Safe with NULL. You are recommended to reset
/// the value to NULL after freeing.
#[no_mangle]
pub unsafe extern "C" fn orz_free_stat(ptr: *mut Stat) {
    if !ptr.is_null() {
        Box::from_raw(ptr);
    }
}
