#![feature(portable_simd)]
#![feature(slice_swap_unchecked)]
#![feature(likely_unlikely)]

pub mod ffi;
pub mod lz;

mod build;
mod coder;
mod huffman;
mod matchfinder;
mod mem;
mod symrank;

use std::{
    io::{Read, Write},
    time::{Duration, Instant},
};

use byteorder::{ReadBytesExt, WriteBytesExt};

use crate::{
    build::{LZ_LENID_SIZE, LZ_MF_BUCKET_ITEM_SIZE, LZ_ROID_SIZE, SYMRANK_NUM_SYMBOLS},
    lz::{LZCfg, LZDecoder, LZEncoder},
};

const LZ_BLOCK_SIZE: usize = (1 << 25) - 1; // 32MB
const LZ_CHUNK_SIZE: usize = 1 << 20; // 1MB
const LZ_MATCH_MAX_LEN: usize = 240; // requires max_len=16n
const LZ_MATCH_MIN_LEN: usize = 4;
const LZ_MF_BUCKET_ITEM_HASH_SIZE: usize = (LZ_MF_BUCKET_ITEM_SIZE as f64 * 1.13) as usize | 1;

#[macro_export]
macro_rules! unchecked {
    ($e:expr) => {{
        #[allow(unused_unsafe)]
        unsafe {
            unchecked_index::unchecked_index($e)
        }
    }};
}

/// Compression size info: source/target sizes.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Stat {
    pub source_size: u64,
    pub target_size: u64,
    pub start_time: Instant,
    pub duration: Duration,
}

impl Stat {
    pub fn new() -> Self {
        Self {
            source_size: 0,
            target_size: 0,
            start_time: Instant::now(),
            duration: Duration::ZERO,
        }
    }

    pub fn log_progress(&mut self, source_size_inc: u64, target_size_inc: u64, is_encode: bool) {
        self.source_size += source_size_inc;
        self.target_size += target_size_inc;
        self.duration = std::time::Instant::now().duration_since(self.start_time);
        let duration_secs =
            self.duration.as_secs() as f64 + self.duration.subsec_nanos() as f64 * 1e-9;
        let mbps = self.source_size as f64 * 1e-6 / duration_secs;

        if is_encode {
            log::info!(
                "encode: {} bytes => {} bytes, {:.3}MB/s",
                source_size_inc,
                target_size_inc,
                mbps
            );
        } else {
            log::info!(
                "decode: {} bytes <= {} bytes, {:.3}MB/s",
                source_size_inc,
                target_size_inc,
                mbps
            );
        }
    }

    pub fn log_finish(&mut self, is_encode: bool) {
        self.duration = Instant::now().duration_since(self.start_time);
        let duration_secs =
            self.duration.as_secs() as f64 + self.duration.subsec_nanos() as f64 * 1e-9;
        let mbps = self.source_size as f64 * 1e-6 / duration_secs;
        log::info!("statistics:");
        log::info!(
            "  size:  {0} bytes {2} {1} bytes",
            self.source_size,
            self.target_size,
            if is_encode { "=>" } else { "<=" },
        );
        log::info!(
            "  ratio: {:.2}%",
            self.target_size as f64 * 100.0 / self.source_size as f64
        );
        log::info!("  speed: {:.3} MB/s", mbps);
        log::info!("  time:  {:.3} sec", duration_secs);
    }
}

/// Reads until EOF or until buffer is filled
fn read_repeatedly<R: Read + ?Sized>(source: &mut R, buf: &mut [u8]) -> std::io::Result<usize> {
    let mut result = 0;
    while result < buf.len() {
        let have_read = source.read(&mut buf[result..])?;
        if have_read == 0 {
            break;
        }
        result += have_read;
    }
    Ok(result)
}

const SBVEC_SENTINEL_LEN: usize = LZ_MATCH_MAX_LEN * 2;
const SBVEC_PREMATCH_LEN: usize = LZ_BLOCK_SIZE / 2;

/// Encode the source into a target ORZ stream.
pub fn encode(source: &mut dyn Read, target: &mut dyn Write, cfg: &LZCfg) -> std::io::Result<Stat> {
    let mut stat = Stat::new();
    let mut lzenc = LZEncoder::new();

    let mut sbvec_buf = vec![0u8; LZ_BLOCK_SIZE + SBVEC_SENTINEL_LEN * 2];
    let mut tbvec_buf = vec![0u8; SBVEC_PREMATCH_LEN * 3];
    let sbvec = &mut sbvec_buf[SBVEC_SENTINEL_LEN..][..LZ_BLOCK_SIZE];
    let tbvec = &mut tbvec_buf;

    loop {
        let sbvec_read_size = read_repeatedly(source, &mut sbvec[SBVEC_PREMATCH_LEN..])?;
        let mut spos = SBVEC_PREMATCH_LEN;
        let mut tpos = 0usize;

        while spos < SBVEC_PREMATCH_LEN + sbvec_read_size {
            let sbvec = &sbvec[..SBVEC_PREMATCH_LEN + sbvec_read_size];
            let (s, t) = unsafe { lzenc.encode(cfg, &sbvec, tbvec.as_mut(), spos) };
            target.write_u32::<byteorder::LE>(t as u32)?;
            stat.target_size += 4;

            target.write_all(&tbvec[..t])?;
            spos = s;
            tpos += t;
        }
        unsafe {
            std::ptr::copy(
                sbvec.as_ptr().add(sbvec.len() - SBVEC_PREMATCH_LEN),
                sbvec.as_mut_ptr(),
                SBVEC_PREMATCH_LEN,
            );
        }
        lzenc.forward(sbvec.len() - SBVEC_PREMATCH_LEN); // reset orz_lz encoder
        stat.log_progress(spos as u64 - SBVEC_PREMATCH_LEN as u64, tpos as u64, true);

        // reached end of file
        if sbvec_read_size < sbvec[SBVEC_PREMATCH_LEN..].len() {
            break;
        }
    }

    // write a empty chunk to mark eof
    target.write_u32::<byteorder::LE>(0u32)?;
    stat.target_size += 4;
    Ok(stat)
}

pub fn decode(target: &mut dyn Read, source: &mut dyn Write) -> std::io::Result<Stat> {
    let mut stat = Stat::new();
    let mut lzdec = LZDecoder::new();

    let mut sbvec_buf = vec![0u8; LZ_BLOCK_SIZE * 2 + SBVEC_SENTINEL_LEN * 2];
    let mut tbvec_buf = vec![0u8; SBVEC_PREMATCH_LEN * 3];
    let sbvec = &mut sbvec_buf[SBVEC_SENTINEL_LEN..][..LZ_BLOCK_SIZE];
    let tbvec = &mut tbvec_buf;

    let mut spos = SBVEC_PREMATCH_LEN;
    let mut tpos = 0usize;
    loop {
        let t = target.read_u32::<byteorder::LE>()? as usize;
        if t >= tbvec.len() {
            return Err(std::io::ErrorKind::InvalidData.into());
        }
        stat.target_size += 4;
        if t == 0 {
            // EOF
            stat.log_progress(spos as u64 - SBVEC_PREMATCH_LEN as u64, tpos as u64, false);
            break;
        }

        if t != 0 {
            target.read_exact(&mut tbvec[..t])?;
            let (s, t) = unsafe {
                lzdec
                    .decode(&tbvec[..t], sbvec.as_mut(), spos)
                    .or(Err(std::io::ErrorKind::InvalidData))?
            };
            source.write_all(&sbvec[spos..s])?;
            spos = s;
            tpos += t;
        }

        if spos >= LZ_BLOCK_SIZE {
            unsafe {
                std::ptr::copy(
                    sbvec.as_ptr().add(sbvec.len() - SBVEC_PREMATCH_LEN),
                    sbvec.as_mut_ptr(),
                    SBVEC_PREMATCH_LEN,
                );
            }
            lzdec.forward(sbvec.len() - SBVEC_PREMATCH_LEN);
            stat.log_progress(spos as u64 - SBVEC_PREMATCH_LEN as u64, tpos as u64, false);
            spos = SBVEC_PREMATCH_LEN;
            tpos = 0usize;
        }
    }
    Ok(stat)
}
