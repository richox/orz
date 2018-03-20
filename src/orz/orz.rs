use orz::lempziv::*;
use orz::matchfinder::*;
use std;

pub struct Statistics {
    pub source_size: u64,
    pub target_size: u64,
}

pub struct Orz {}
impl Orz {
    pub fn encode(
        source: &mut std::io::Read,
        target: &mut std::io::Write,
        cfg: &LZCfg,
    ) -> std::io::Result<Statistics> {
        let time_start = std::time::SystemTime::now();
        let mut statistics = Statistics {
            source_size: 0,
            target_size: 0,
        };

        target.write_all(&[
            // write block size
            (cfg.block_size / 16777216 % 256) as u8,
            (cfg.block_size / 65536 % 256) as u8,
            (cfg.block_size / 256 % 256) as u8,
            (cfg.block_size / 1 % 256) as u8,
        ])?;
        statistics.target_size += 4;

        let sbvec = &mut vec![0u8; cfg.block_size + LZ_MATCH_MAX_LEN * 4][ // with sentinel
            (LZ_MATCH_MAX_LEN * 2) .. (LZ_MATCH_MAX_LEN * 2 + cfg.block_size)
        ];
        let tbvec = &mut vec![0u8; LZ_CHUNK_TARGET_SIZE];
        let lzenc = &mut LZEncoder::new();

        loop {
            let sbvec_read_size = {
                let mut total_read_size = 0usize;
                while total_read_size < sbvec.len() {
                    let read_size = source.read(&mut sbvec[total_read_size..])?;
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

            let mut spos = 0usize;
            let mut tpos = 0usize;

            while spos < sbvec_read_size {
                let (s, t) = unsafe { lzenc.encode(cfg, &sbvec[..sbvec_read_size], tbvec, spos) };
                target.write_all(&[(t >> 0) as u8, (t >> 8) as u8, (t >> 16) as u8])?;
                statistics.target_size += 3;

                target.write_all(&tbvec[..t])?;
                spos = s;
                tpos = tpos + t;
            }
            statistics.source_size += spos as u64;
            statistics.target_size += tpos as u64;

            let time_end = std::time::SystemTime::now();
            let duration = time_end.duration_since(time_start).unwrap();
            let duration_secs = duration.as_secs() as f64 + duration.subsec_nanos() as f64 * 1e-9;
            eprintln!(
                "encode: {} bytes => {} bytes, {:.3}MB/s",
                spos,
                tpos,
                statistics.source_size as f64 * 1e-6 / duration_secs
            );
            lzenc.reset(); // reset lempziv encoder
        }
        return Ok(statistics);
    }

    pub fn decode(
        target: &mut std::io::Read,
        source: &mut std::io::Write,
    ) -> std::io::Result<Statistics> {
        let time_start = std::time::SystemTime::now();
        let mut statistics = Statistics {
            source_size: 0,
            target_size: 0,
        };

        let block_size_buf = &mut [0u8; 4];
        target
            .read_exact(block_size_buf)
            .or(Err(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "invalid block size",
            )))?;
        let block_size = block_size_buf[0] as usize * 16777216 + block_size_buf[1] as usize * 65536
            + block_size_buf[2] as usize * 256
            + block_size_buf[3] as usize * 1;
        statistics.target_size += 4;

        let sbvec = &mut vec![0u8; block_size + LZ_MATCH_MAX_LEN * 4][ // with sentinel
            (LZ_MATCH_MAX_LEN * 2) .. (LZ_MATCH_MAX_LEN * 2 + block_size)
        ];
        let tbvec = &mut vec![0u8; LZ_CHUNK_TARGET_SIZE];
        let lzdec = &mut LZDecoder::new();

        let mut spos = 0usize;
        let mut tpos = 0usize;
        loop {
            let mut chunk_header_buf = [0u8; 3];
            let eof = target.read(&mut chunk_header_buf).and_then(|size| {
                statistics.target_size += size as u64;
                match size {
                    3 => Ok(false),
                    0 => Ok(true),
                    _ => Err(std::io::Error::new(
                        std::io::ErrorKind::UnexpectedEof,
                        "missing chunk header",
                    )),
                }
            })?;

            if !eof {
                let t = (chunk_header_buf[0] as usize) << 0 | (chunk_header_buf[1] as usize) << 8
                    | (chunk_header_buf[2] as usize) << 16;
                if t >= LZ_CHUNK_TARGET_SIZE {
                    Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        "invalid chunk header",
                    ))?;
                }
                target.read_exact(&mut tbvec[..t])?;

                let (s, t) = unsafe {
                    lzdec
                        .decode(&tbvec[..t], &mut sbvec[..block_size], spos)
                        .or(Err(std::io::Error::new(
                            std::io::ErrorKind::InvalidData,
                            "invalid chunk data",
                        )))
                }?;
                source.write_all(&sbvec[spos..s])?;
                spos = s;
                tpos = tpos + t;
            }

            if spos == block_size || eof {
                statistics.source_size += spos as u64;
                statistics.target_size += tpos as u64;
                let time_end = std::time::SystemTime::now();
                let duration = time_end.duration_since(time_start).unwrap();
                let duration_secs =
                    duration.as_secs() as f64 + duration.subsec_nanos() as f64 * 1e-9;
                eprintln!(
                    "decode: {} bytes <= {} bytes, {:.3}MB/s",
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
