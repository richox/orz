extern crate byteorder;
extern crate chrono;
extern crate structopt;

mod _constants;
mod bits;
mod aux;
mod huffman;
mod lz;
mod matchfinder;
mod mtf;

use byteorder::ReadBytesExt;
use byteorder::WriteBytesExt;
use chrono::Utc;
use structopt::*;
use self::lz::LZCfg;
use self::lz::LZDecoder;
use self::lz::LZEncoder;

use _constants::lz_roid_array::LZ_ROID_SIZE;
use _constants::lz_roid_array::LZ_MF_BUCKET_ITEM_SIZE;

const LZ_CHUNK_SIZE: usize = 393216;
const LZ_CONTEXT_BUCKET_SIZE: usize = 256;
const LZ_MATCH_MAX_LEN: usize = 255;
const LZ_MATCH_MIN_LEN: usize = 4;
const LZ_MF_BUCKET_ITEM_HASH_SIZE: usize = 8192;

struct Stat {
    pub source_size: u64,
    pub target_size: u64,
}

fn encode(source: &mut std::io::Read, target: &mut std::io::Write, cfg: &LZCfg) -> std::io::Result<Stat> {
    let block_size = cfg.block_size * 1048576;
    let start_time = Utc::now();
    let mut lzenc = LZEncoder::new();
    let mut statistics = Stat {source_size: 0, target_size: 0};

    target.write_u32::<byteorder::BE>(block_size as u32)?;
    statistics.target_size += 4;

    const SBVEC_SENTINEL_LEN: usize = LZ_MATCH_MAX_LEN * 2;
    let sbvec = &mut vec![0u8; block_size + SBVEC_SENTINEL_LEN * 2][SBVEC_SENTINEL_LEN ..][.. block_size];
    let tbvec = &mut vec![0u8; sbvec.len() * 3];

    loop {
        let sbvec_read_size = {
            let mut total_read_size = 0usize;
            while total_read_size < sbvec.len() {
                let read_size = source.read(&mut sbvec[total_read_size ..])?;
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
            let (s, t) = unsafe {
                lzenc.encode(cfg, &sbvec[ .. sbvec_read_size], tbvec, spos)
            };
            target.write_u32::<byteorder::BE>(t as u32)?;
            statistics.target_size += 4;

            target.write_all(&tbvec[ .. t])?;
            spos = s;
            tpos = tpos + t;
        }
        statistics.source_size += spos as u64;
        statistics.target_size += tpos as u64;
        let duration_ms = Utc::now().signed_duration_since(start_time).num_milliseconds();
        let mbps = statistics.source_size as f64 * 1e-6 / (duration_ms as f64 / 1000.0);

        eprintln!("encode: {} bytes => {} bytes, {:.3}MB/s", spos, tpos, mbps);
        lzenc.reset(); // reset orz_lz encoder
    }

    // write a empty chunk to mark eof
    target.write_u32::<byteorder::BE>(0u32)?;
    statistics.target_size += 4;
    Ok(statistics)
}

fn decode(target: &mut std::io::Read, source: &mut std::io::Write) -> std::io::Result<Stat> {
    let start_time = Utc::now();
    let mut lzdec = LZDecoder::new();
    let mut statistics = Stat {source_size: 0, target_size: 0};

    let block_size = target.read_u32::<byteorder::BE>()? as usize;
    statistics.target_size += 4;

    const SBVEC_SENTINEL_LEN: usize = LZ_MATCH_MAX_LEN * 2;
    let sbvec = &mut vec![0u8; block_size + SBVEC_SENTINEL_LEN * 2][SBVEC_SENTINEL_LEN ..][.. block_size];
    let tbvec = &mut vec![0u8; sbvec.len() * 3];

    let mut spos = 0usize;
    let mut tpos = 0usize;
    loop {
        let t = target.read_u32::<byteorder::BE>()? as usize;
        if t >= tbvec.len() {
            Err(std::io::ErrorKind::InvalidData)?;
        }
        statistics.target_size += 4;

        if t != 0 {
            target.read_exact(&mut tbvec[ .. t])?;
            let (s, t) = unsafe {
                lzdec.decode(&tbvec[ .. t], sbvec, spos).or(Err(std::io::ErrorKind::InvalidData))?
            };
            source.write_all(&sbvec[spos .. s])?;
            spos = s;
            tpos = t + tpos;
        }

        if spos >= block_size || t == 0 {
            statistics.source_size += spos as u64;
            statistics.target_size += tpos as u64;

            let duration_ms = Utc::now().signed_duration_since(start_time).num_milliseconds();
            let mbps = statistics.source_size as f64 * 1e-6 / (duration_ms as f64 / 1000.0);
            eprintln!("decode: {} bytes <= {} bytes, {:.3}MB/s", spos, tpos, mbps);
            if t == 0 {
                break;
            }
            spos = 0;
            tpos = 0;
            lzdec.reset();
        }
    }
    Ok(statistics)
}

#[derive(StructOpt, Debug)]
#[structopt(name = "orz", about = "an optimized ROLZ data compressor")]
enum Opt {
    #[structopt(name = "encode", about = "Encode")]
    Encode {
        #[structopt(short = "b", default_value = "16")] /// set compression block size (MB)
        block_size: usize,
        #[structopt(short = "l", default_value = "4")] /// set compression level (0..4)
        level: u8,
        #[structopt(parse(from_os_str))] /// source file name, default to stdin
        ipath: Option<std::path::PathBuf>,
        #[structopt(parse(from_os_str))] /// target file name, default to stdout
        opath: Option<std::path::PathBuf>,
    },

    #[structopt(name = "decode", about = "Decode")]
    Decode {
        #[structopt(parse(from_os_str))] /// source file name, default to stdin
        ipath: Option<std::path::PathBuf>,
        #[structopt(parse(from_os_str))] /// target file name, default to stdout
        opath: Option<std::path::PathBuf>,
    },
}

fn main() -> Result<(), Box<std::error::Error>> {
    let start_time = Utc::now();
    let args = Opt::from_args();

    let get_ifile = |ipath| -> Result<Box<std::io::Read>, Box<std::error::Error>> {
        Ok(match ipath {
            &Some(ref p) => Box::new(std::fs::File::open(p)?),
            &None => Box::new(std::io::stdin()),
        })
    };
    let get_ofile = |opath| -> Result<Box<std::io::Write>, Box<std::error::Error>> {
        Ok(match opath {
            &Some(ref p) => Box::new(std::fs::File::create(p)?),
            &None => Box::new(std::io::stdout()),
        })
    };

    // encode/decode
    let statistics = match &args {
        &Opt::Encode {block_size, level, ref ipath, ref opath} => {
            if block_size <= 0 {
                Err(format!("invalid block size: {}", block_size))?;
            }
            encode(
                &mut get_ifile(ipath)?,
                &mut get_ofile(opath)?,
                &match level {
                    0 => LZCfg {block_size, match_depth:  3, lazy_match_depth1: 1, lazy_match_depth2: 1},
                    1 => LZCfg {block_size, match_depth:  5, lazy_match_depth1: 2, lazy_match_depth2: 1},
                    2 => LZCfg {block_size, match_depth:  8, lazy_match_depth1: 3, lazy_match_depth2: 2},
                    3 => LZCfg {block_size, match_depth: 13, lazy_match_depth1: 5, lazy_match_depth2: 3},
                    4 => LZCfg {block_size, match_depth: 21, lazy_match_depth1: 8, lazy_match_depth2: 5},
                    _ => Err(format!("invalid level: {}", level))?,
                },
            ).or_else(|e| Err(format!("encoding failed: {}", e)))?
        }

        &Opt::Decode {ref ipath, ref opath} => {
            decode(
                &mut get_ifile(ipath)?,
                &mut get_ofile(opath)?,
            ).or_else(|e| Err(format!("decoding failed: {}", e)))?
        }
    };

    // dump statistics
    let duration_ms = Utc::now().signed_duration_since(start_time).num_milliseconds();
    eprintln!("statistics:");
    eprintln!("  size:  {0} bytes {2} {1} bytes",
        statistics.source_size,
        statistics.target_size,
        match &args {
            &Opt::Encode {..} => "=>",
            &Opt::Decode {..} => "<=",
        });
    eprintln!("  ratio: {:.2}%", statistics.target_size as f64 * 100.0 / statistics.source_size as f64);
    eprintln!("  time:  {:.3} sec", duration_ms as f64 / 1000.0);
    Ok(())
}
