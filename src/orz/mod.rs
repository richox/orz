mod constants;
mod bits;
mod huff;
mod matchfinder;
mod lempziv;
mod mtf;

use std;
use self::lempziv::*;
pub use self::lempziv::LZCfg;

pub struct Statistics {
    pub source_size: u64,
    pub target_size: u64,
}

pub struct Orz {}
impl Orz {
    pub fn encode(source: &mut std::io::Read, target: &mut std::io::Write, cfg: &LZCfg) -> std::io::Result<Statistics> {
        let time_start = std::time::SystemTime::now();
        let mut sbvec = vec![0u8; LZ_BLOCK_SIZE];
        let mut tbvec = vec![0u8; LZ_CHUNK_TARGET_SIZE];
        let mut lzenc = LZEncoder::new();
        let mut statistics = Statistics {
            source_size: 0,
            target_size: 0,
        };

        loop {
            let sbvec_read_size = source.read(&mut sbvec)?;
            if sbvec_read_size == 0 {
                break;
            }
            let mut spos = 0usize;
            let mut tpos = 0usize;

            while spos < sbvec_read_size {
                let (s, t) = unsafe {
                    lzenc.encode(cfg, &sbvec[ .. sbvec_read_size], &mut tbvec, spos)
                };
                target.write_all(&[
                    (t >>  0) as u8,
                    (t >>  8) as u8,
                    (t >> 16) as u8,
                ])?;
                target.write_all(&tbvec[ .. t])?;
                spos = s;
                tpos = tpos + t;
            }
            statistics.source_size += spos as u64;
            statistics.target_size += tpos as u64;
            let time_end = std::time::SystemTime::now();
            let duration = time_end.duration_since(time_start).unwrap();
            let duration_secs = duration.as_secs() as f64 + duration.subsec_nanos() as f64 * 1e-9;
            eprintln!("encode: {} bytes => {} bytes, {:.3}MB/s",
                spos,
                tpos,
                statistics.source_size as f64 * 1e-6 / duration_secs
            );
            lzenc.reset(); // reset lempziv encoder
        }
        return Ok(statistics);
    }

    pub fn decode(target: &mut std::io::Read, source: &mut std::io::Write) -> std::io::Result<Statistics> {
        let time_start = std::time::SystemTime::now();
        let mut sbvec = vec![0u8; LZ_BLOCK_SIZE + 512 /* as sentinel */];
        let mut tbvec = vec![0u8; LZ_CHUNK_TARGET_SIZE];
        let mut lzdec = LZDecoder::new();
        let mut statistics = Statistics {
            source_size: 0,
            target_size: 0,
        };

        let mut spos = 0usize;
        let mut tpos = 0usize;
        loop {
            let mut chunk_header_buf = [0u8; 3];
            let eof = target.read(&mut chunk_header_buf).and_then(|size|
                match size {
                    3 => Ok(false),
                    0 => Ok(true),
                    _ => Err(std::io::Error::new(
                        std::io::ErrorKind::UnexpectedEof,
                        "missing chunk header",
                    )),
                })?;

            if !eof {
                let t =
                    (chunk_header_buf[0] as usize) <<  0 |
                    (chunk_header_buf[1] as usize) <<  8 |
                    (chunk_header_buf[2] as usize) << 16;
                if t >= LZ_CHUNK_TARGET_SIZE {
                    Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "invalid chunk header"))?;
                }
                target.read_exact(&mut tbvec[ .. t])?;

                let (s, t) = unsafe {
                    lzdec.decode(&tbvec[ .. t], &mut sbvec[ .. LZ_BLOCK_SIZE], spos).or(
                        Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "invalid chunk data")))
                }?;
                source.write_all(&sbvec[spos .. s])?;
                spos = s;
                tpos = tpos + t;
            }

            if spos == LZ_BLOCK_SIZE || eof {
                statistics.source_size += spos as u64;
                statistics.target_size += tpos as u64;
                let time_end = std::time::SystemTime::now();
                let duration = time_end.duration_since(time_start).unwrap();
                let duration_secs = duration.as_secs() as f64 + duration.subsec_nanos() as f64 * 1e-9;
                eprintln!("decode: {} bytes <= {} bytes, {:.3}MB/s",
                    spos,
                    tpos,
                    statistics.source_size as f64 * 1e-6 / duration_secs
                );
                if eof {
                    break;
                }
                spos = 0;
                tpos = 0;
                lzdec.reset();
            }
        }
        return Ok(statistics);
    }
}
