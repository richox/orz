// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// https://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

#![feature(portable_simd)]
#![feature(likely_unlikely)]

// pub mod ffi;
mod coder;
mod huffman;
mod ioutil;
mod lz;
mod matcher;
mod mem;
mod progress;
mod symrank;

use std::io::{Read, Result, Write};

pub use ioutil::{CountRead, CountWrite};
pub use lz::LZCfg;
pub use progress::{ProgressLogger, SilentProgressLogger, SimpleProgressLogger};

use crate::{
    ioutil::{ReadExt, WriteExt},
    lz::{LZ_MF_BUCKET_ITEM_SIZE, LZDecoder, LZEncoder, SYMRANK_NUM_SYMBOLS},
};

const LZ_BLOCK_SIZE: usize = (1 << 25) - 1; //32MB
const LZ_CHUNK_SIZE: usize = 1 << 20; // 1MB
const LZ_MATCH_MAX_LEN: usize = 240; // requires max_len=16n
const LZ_MATCH_MIN_LEN: usize = 4;

#[macro_export]
macro_rules! unchecked {
    ($e:expr) => {{ unsafe { unchecked_index::unchecked_index($e) } }};
}

/// Reads until EOF or until buffer is filled
fn read_repeatedly<R: Read + ?Sized>(source: &mut R, buf: &mut [u8]) -> Result<usize> {
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
pub fn encode<R: Read, W: Write>(
    source: &mut CountRead<R>,
    target: &mut CountWrite<W>,
    cfg: &LZCfg,
    progress_logger: &mut Box<dyn ProgressLogger>,
) -> Result<()> {
    let mut lzenc = LZEncoder::new();
    progress_logger.set_is_encode(true);

    let mut sbvec_buf = vec![0u8; LZ_BLOCK_SIZE + SBVEC_SENTINEL_LEN * 2];
    let mut tbvec_buf = vec![0u8; SBVEC_PREMATCH_LEN * 3];
    let sbvec = &mut sbvec_buf[SBVEC_SENTINEL_LEN..][..LZ_BLOCK_SIZE];
    let tbvec = &mut tbvec_buf;

    while let sbvec_read_size = read_repeatedly(source, &mut sbvec[SBVEC_PREMATCH_LEN..])?
        && sbvec_read_size > 0
    {
        let mut spos = SBVEC_PREMATCH_LEN;
        while spos < SBVEC_PREMATCH_LEN + sbvec_read_size {
            let sbvec = &sbvec[..SBVEC_PREMATCH_LEN + sbvec_read_size];
            let (s, t) = lzenc.encode(cfg, &sbvec, tbvec.as_mut(), spos);
            target.write_len(t)?;
            target.write_all(&tbvec[..t])?;
            spos = s;
        }
        sbvec.copy_within(sbvec.len() - SBVEC_PREMATCH_LEN..sbvec.len(), 0);
        lzenc.forward(sbvec.len() - SBVEC_PREMATCH_LEN); // reset orz_lz encoder
        progress_logger.log(source.count(), target.count());
    }

    // write an empty chunk to mark eof
    target.write_len(0)?;
    progress_logger.finish(source.count(), target.count());
    Ok(())
}

pub fn decode<R: Read, W: Write>(
    target: &mut CountRead<R>,
    source: &mut CountWrite<W>,
    progress_logger: &mut Box<dyn ProgressLogger>,
) -> Result<()> {
    let mut lzdec = LZDecoder::new();
    progress_logger.set_is_encode(false);

    let mut sbvec_buf = vec![0u8; LZ_BLOCK_SIZE * 2 + SBVEC_SENTINEL_LEN * 2];
    let mut tbvec_buf = vec![0u8; SBVEC_PREMATCH_LEN * 3];
    let sbvec = &mut sbvec_buf[SBVEC_SENTINEL_LEN..][..LZ_BLOCK_SIZE];
    let tbvec = &mut tbvec_buf;

    let mut spos = SBVEC_PREMATCH_LEN;
    while let t = target.read_len()?
        && t != 0
    {
        if t >= tbvec.len() {
            return Err(std::io::ErrorKind::InvalidData.into());
        }
        target.read_exact(&mut tbvec[..t])?;

        let spos_end = lzdec.decode(&tbvec[..t], sbvec.as_mut(), spos)?;
        source.write_all(&sbvec[spos..spos_end])?;
        spos = spos_end;

        if spos >= LZ_BLOCK_SIZE {
            sbvec.copy_within(sbvec.len() - SBVEC_PREMATCH_LEN..sbvec.len(), 0);
            lzdec.forward(sbvec.len() - SBVEC_PREMATCH_LEN);
            progress_logger.log(target.count(), source.count());
            spos = SBVEC_PREMATCH_LEN;
        }
    }
    progress_logger.finish(target.count(), source.count());
    Ok(())
}
