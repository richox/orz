pub mod ffi;
pub mod lz;

mod bits;
mod build;
mod byteslice;
mod huffman;
mod matchfinder;
mod mem;
mod symrank;

use std::io::Read;
use std::io::Write;
use std::time::Instant;

use crate::build::LZ_LENID_SIZE;
use crate::build::LZ_MF_BUCKET_ITEM_SIZE;
use crate::build::LZ_ROID_SIZE;
use crate::build::SYMRANK_NUM_SYMBOLS;
use crate::lz::LZCfg;
use crate::lz::LZDecoder;
use crate::lz::LZEncoder;

use byteorder::ReadBytesExt;
use byteorder::WriteBytesExt;

const LZ_BLOCK_SIZE: usize = (1 << 25) - 1; // 32MB
const LZ_CHUNK_SIZE: usize = 1 << 20; // 1MB
const LZ_MATCH_MAX_LEN: usize = 240; // requires max_len=16n
const LZ_MATCH_MIN_LEN: usize = 4;
const LZ_MF_BUCKET_ITEM_HASH_SIZE: usize = (LZ_MF_BUCKET_ITEM_SIZE as f64 * 1.13) as usize | 1;

/// Compression size info: source/target sizes.
#[repr(C)]
pub struct Stat {
    pub source_size: u64,
    pub target_size: u64,
}
const SBVEC_SENTINEL_LEN: usize = LZ_MATCH_MAX_LEN * 2;
const SBVEC_PREMATCH_LEN: usize = LZ_BLOCK_SIZE / 2;
const SBVEC_POSTMATCH_LEN: usize = LZ_BLOCK_SIZE - SBVEC_PREMATCH_LEN;

/// Encode the source into a target ORZ stream.
pub fn encode(source: &mut dyn Read, target: &mut dyn Write, cfg: &LZCfg) -> std::io::Result<Stat> {
    let start_time = Instant::now();
    let sbvec = &mut Box::new([0u8; LZ_BLOCK_SIZE + SBVEC_SENTINEL_LEN])[..LZ_BLOCK_SIZE];
    let tbvec = &mut Box::new([0u8; SBVEC_PREMATCH_LEN * 3]);
    let mut lzenc = LZEncoder::default();
    let mut statistics = Stat {
        source_size: 0,
        target_size: 0,
    };

    // writer version
    let version_bytes = env!("CARGO_PKG_VERSION").as_bytes();
    let mut version_str_buf = [0u8; 10]; // to store version string like xx.yy.zz
    version_str_buf[..version_bytes.len()].copy_from_slice(version_bytes);
    target.write_all(&version_str_buf)?;
    statistics.target_size += version_str_buf.len() as u64;

    loop {
        let sbvec_read_size = {
            let mut total_read_size = 0usize;
            while SBVEC_PREMATCH_LEN + total_read_size < sbvec.len() {
                let read_size = source.read(&mut sbvec[SBVEC_PREMATCH_LEN + total_read_size..])?;
                if read_size == 0 {
                    break;
                }
                total_read_size += read_size;
            }
            total_read_size
        };
        if sbvec_read_size == 0 {
            break;
        }

        let mut spos = SBVEC_PREMATCH_LEN;
        let mut tpos = 0usize;

        while spos < SBVEC_PREMATCH_LEN + sbvec_read_size {
            let (s, t) = unsafe {
                lzenc.encode(
                    cfg,
                    &sbvec[..SBVEC_PREMATCH_LEN + sbvec_read_size],
                    &mut tbvec[..],
                    spos,
                )
            };
            target.write_u32::<byteorder::LE>(t as u32)?;
            statistics.target_size += 4;

            target.write_all(&tbvec[..t])?;
            spos = s;
            tpos += t;
        }
        statistics.source_size += spos as u64 - SBVEC_PREMATCH_LEN as u64;
        statistics.target_size += tpos as u64;
        let duration = std::time::Instant::now().duration_since(start_time);
        let duration_secs = duration.as_secs() as f64 + duration.subsec_nanos() as f64 * 1e-9;
        let mbps = statistics.source_size as f64 * 1e-6 / duration_secs;

        log::info!(
            "encode: {} bytes => {} bytes, {:.3}MB/s",
            spos - SBVEC_PREMATCH_LEN,
            tpos,
            mbps
        );
        unsafe {
            std::ptr::copy(
                sbvec.as_ptr().add(SBVEC_POSTMATCH_LEN),
                sbvec.as_mut_ptr(),
                SBVEC_PREMATCH_LEN,
            );
        }
        lzenc.forward(SBVEC_POSTMATCH_LEN); // reset orz_lz encoder
    }

    // write a empty chunk to mark eof
    target.write_u32::<byteorder::LE>(0u32)?;
    statistics.target_size += 4;
    Ok(statistics)
}

pub fn decode(target: &mut dyn Read, source: &mut dyn Write) -> std::io::Result<Stat> {
    let start_time = Instant::now();
    let sbvec = &mut Box::new([0u8; LZ_BLOCK_SIZE + SBVEC_SENTINEL_LEN])[..LZ_BLOCK_SIZE];
    let tbvec = &mut Box::new([0u8; SBVEC_PREMATCH_LEN * 3]);
    let mut lzdec = LZDecoder::default();
    let mut statistics = Stat {
        source_size: 0,
        target_size: 0,
    };
    let mut spos = SBVEC_PREMATCH_LEN;
    let mut tpos = 0usize;

    // check version
    let mut version_bytes = [0u8; 10];
    target.read_exact(&mut version_bytes)?;
    let version_str = std::str::from_utf8(&version_bytes)
        .map_err(|_| {
            std::io::Error::new(std::io::ErrorKind::InvalidData, "invalid utf-8 version str")
        })?
        .trim_end_matches('\u{0}');

    if !version_str.to_owned().eq(env!("CARGO_PKG_VERSION")) {
        log::warn!(
            "version mismatched ({} vs {}), decoding may not work correctly",
            version_str,
            env!("CARGO_PKG_VERSION")
        );
    }
    statistics.target_size += version_bytes.len() as u64;

    loop {
        let t = target.read_u32::<byteorder::LE>()? as usize;
        if t >= tbvec.len() {
            return Err(std::io::ErrorKind::InvalidData.into());
        }
        statistics.target_size += 4;

        if t != 0 {
            target.read_exact(&mut tbvec[..t])?;
            let (s, t) = unsafe {
                lzdec
                    .decode(&tbvec[..t], &mut *sbvec, spos)
                    .or(Err(std::io::ErrorKind::InvalidData))?
            };
            source.write_all(&sbvec[spos..s])?;
            spos = s;
            tpos += t;
        }

        if spos >= LZ_BLOCK_SIZE || t == 0 {
            statistics.source_size += spos as u64 - SBVEC_PREMATCH_LEN as u64;
            statistics.target_size += tpos as u64;

            let duration = Instant::now().duration_since(start_time);
            let duration_secs = duration.as_secs() as f64 + duration.subsec_nanos() as f64 * 1e-9;
            let mbps = statistics.source_size as f64 * 1e-6 / duration_secs;

            log::info!(
                "decode: {} bytes <= {} bytes, {:.3}MB/s",
                spos - SBVEC_PREMATCH_LEN,
                tpos,
                mbps
            );
            if t == 0 {
                break;
            }
            spos = SBVEC_PREMATCH_LEN;
            tpos = 0;
            unsafe {
                std::ptr::copy(
                    sbvec.as_ptr().add(SBVEC_POSTMATCH_LEN),
                    sbvec.as_mut_ptr(),
                    SBVEC_PREMATCH_LEN,
                );
            }
            lzdec.forward(SBVEC_POSTMATCH_LEN);
        }
    }
    Ok(statistics)
}

#[macro_export]
macro_rules! assert_unchecked {
    ($cond:expr) => {
        if !$cond {
            if cfg!(debug_assertions) {
                panic!(
                    "Fatal error: assertion `{}` failed: this is a bug and a safety issue!",
                    stringify!($cond)
                );
            }
            unsafe { std::hint::unreachable_unchecked() };
        }
    };
}
