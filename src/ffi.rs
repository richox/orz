use super::lz::LZCfg;
use super::{decode, encode, Stat};
use libc::size_t;
use std::ptr;
use std::slice;

// C FFI forwarders
/// Encode a buffer to a buffer.
/// No pointers may be NULL at this point.
///
/// Returns a pointer to a Stat structure if successful; otherwise
/// returns NULL.
///
/// # Safety
/// If your input sizes are bad, expect me to run out of bounds.
#[no_mangle]
pub unsafe extern "C" fn orz_encode(
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
        match result {
            Ok(r) => {
                let raw = Box::new(r);
                Box::into_raw(raw)
            }
            Err(_) => ptr::null(),
        }
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
pub unsafe extern "C" fn orz_decode(
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
        match result {
            Ok(r) => {
                let raw = Box::new(r);
                Box::into_raw(raw)
            }
            Err(_) => ptr::null(),
        }
    }
}

/// Let the rust system take the pointer back.
///
/// # Safety
/// Actually fine.
#[no_mangle]
pub unsafe extern "C" fn orz_free_stat(ptr: *mut Stat) {
    Box::from_raw(ptr);
}
