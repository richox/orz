pub mod ffi;
pub mod lz;

mod bit_queue;
mod build;
mod huffman;
mod matchfinder;
mod mem;
mod symrank;

use std::io::Read;
use std::io::Write;
use std::time::Duration;
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
use smart_default::SmartDefault;

const LZ_BLOCK_SIZE: usize = (1 << 25) - 1; // 32MB
const LZ_CHUNK_SIZE: usize = 1 << 20; // 1MB
const LZ_MATCH_MAX_LEN: usize = 240; // requires max_len=16n
const LZ_MATCH_MIN_LEN: usize = 4;
const LZ_MF_BUCKET_ITEM_HASH_SIZE: usize = (LZ_MF_BUCKET_ITEM_SIZE as f64 * 1.13) as usize | 1;

/// Compression size info: source/target sizes.
#[repr(C)]
#[derive(Clone, Copy, SmartDefault)]
pub struct Stat {
    pub source_size: u64,
    pub target_size: u64,
    #[default(_code = "Instant::now()")] pub start_time: Instant,
    pub duration: Duration,
}
impl Stat {
    pub fn log_progress(&mut self, source_size_inc: u64, target_size_inc: u64, is_encode: bool) {
        self.source_size += source_size_inc;
        self.target_size += target_size_inc;
        self.duration = std::time::Instant::now().duration_since(self.start_time);
        let duration_secs = self.duration.as_secs() as f64 + self.duration.subsec_nanos() as f64 * 1e-9;
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
        let duration_secs = self.duration.as_secs() as f64 + self.duration.subsec_nanos() as f64 * 1e-9;
        let mbps = self.source_size as f64 * 1e-6 / duration_secs;
        log::info!("statistics:");
        log::info!(
            "  size:  {0} bytes {2} {1} bytes",
            self.source_size,
            self.target_size,
            if is_encode {"=>"} else {"<="},
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
    let mut stat = Stat::default();
    let mut lzenc = LZEncoder::default();

    #[allow(unused_allocation)]
    let sbvec = &mut Box::new([0u8; LZ_BLOCK_SIZE + SBVEC_SENTINEL_LEN])[..LZ_BLOCK_SIZE];
    #[allow(unused_allocation)]
    let tbvec = &mut Box::new([0u8; SBVEC_PREMATCH_LEN * 3]);

    stat.target_size += write_version(target)? as u64;
    loop {
        let sbvec_read_size = read_repeatedly(source, &mut sbvec[SBVEC_PREMATCH_LEN..])?;
        let mut spos = SBVEC_PREMATCH_LEN;
        let mut tpos = 0usize;

        while spos < SBVEC_PREMATCH_LEN + sbvec_read_size {
            let sbvec = &sbvec[..SBVEC_PREMATCH_LEN + sbvec_read_size];
            let (s, t) = unsafe {
                lzenc.encode(cfg, &sbvec, tbvec.as_mut(), spos)
            };
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
        stat.log_progress(
            spos as u64 - SBVEC_PREMATCH_LEN as u64,
            tpos as u64,
            true,
        );

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
    let mut stat = Stat::default();
    let mut lzdec = LZDecoder::default();

    #[allow(unused_allocation)]
    let sbvec = &mut Box::new([0u8; LZ_BLOCK_SIZE + SBVEC_SENTINEL_LEN])[..LZ_BLOCK_SIZE];
    #[allow(unused_allocation)]
    let tbvec = &mut Box::new([0u8; SBVEC_PREMATCH_LEN * 3]);

    stat.target_size += check_version(target)? as u64;
    let mut spos = SBVEC_PREMATCH_LEN;
    let mut tpos = 0usize;
    loop {
        let t = target.read_u32::<byteorder::LE>()? as usize;
        if t >= tbvec.len() {
            return Err(std::io::ErrorKind::InvalidData.into());
        }
        stat.target_size += 4;
        if t == 0 { // EOF
            stat.log_progress(
                spos as u64 - SBVEC_PREMATCH_LEN as u64,
                tpos as u64,
                false,
            );
            break;
        }

        if t != 0 {
            target.read_exact(&mut tbvec[..t])?;
            let (s, t) = unsafe {
                lzdec.decode(&tbvec[..t], sbvec.as_mut(), spos)
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
            stat.log_progress(
                spos as u64 - SBVEC_PREMATCH_LEN as u64,
                tpos as u64,
                false,
            );
            spos = SBVEC_PREMATCH_LEN;
            tpos = 0usize;
        }
    }
    Ok(stat)
}

fn write_version(target: &mut dyn Write) -> std::io::Result<usize> {
    let version_bytes = env!("CARGO_PKG_VERSION").as_bytes();
    let mut version_str_buf = [0u8; 10]; // to store version string like xx.yy.zz
    version_str_buf[..version_bytes.len()].copy_from_slice(version_bytes);
    target.write_all(&version_str_buf)?;
    Ok(version_str_buf.len())
}

fn check_version(target: &mut dyn Read) -> std::io::Result<usize> {
    let current_version_str = env!("CARGO_PKG_VERSION");
    let mut version_bytes = [0u8; 10];
    target.read_exact(&mut version_bytes)?;
    let version_str = std::str::from_utf8(&version_bytes)
        .map_err(|_| {
            std::io::Error::new(std::io::ErrorKind::InvalidData, "invalid utf-8 version str")
        })?
    .trim_end_matches('\u{0}');

    // print a warning message rather than exit the decompression
    if !version_str.to_owned().eq(current_version_str) {
        log::warn!(
            "version mismatched ({} vs {}), decoding may not work correctly",
            version_str,
            current_version_str,
        );
    }
    Ok(version_bytes.len())
}
