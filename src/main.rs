#![feature(nll)]
#[macro_use] extern crate structopt;
extern crate byteorder;
extern crate unchecked_index;

mod _constants;
mod bits;
mod auxility;
mod huffman;
mod lz;
mod matchfinder;
mod mem;
mod mtf;

use byteorder::ReadBytesExt;
use byteorder::WriteBytesExt;
use self::lz::LZCfg;
use self::lz::LZDecoder;
use self::lz::LZEncoder;

use _constants::lz_roid_array::LZ_ROID_SIZE;
use _constants::lz_roid_array::LZ_MF_BUCKET_ITEM_SIZE;

const LZ_BLOCK_SIZE: usize = 16777215;
const LZ_PREMATCH_SIZE: usize = LZ_BLOCK_SIZE / 2;
const LZ_CHUNK_SIZE: usize = 524288;
const LZ_MATCH_MAX_LEN: usize = 251; // 248+2+1
const LZ_MATCH_MIN_LEN: usize = 4;
const LZ_MF_BUCKET_ITEM_HASH_SIZE: usize = (LZ_MF_BUCKET_ITEM_SIZE as f64 * 1.33) as usize;

struct Stat {
    pub source_size: u64,
    pub target_size: u64,
}

macro_rules! elog {
    ($silent:expr, $($vargs:tt)*) => {
        if !$silent {
            eprintln!($($vargs)*);
        }
    }
}

fn encode(
    is_silent: bool,
    source: &mut dyn std::io::Read,
    target: &mut dyn std::io::Write,
    cfg: &LZCfg,
) -> std::io::Result<Stat> {

    let start_time = std::time::Instant::now();
    let mut lzenc = LZEncoder::new();
    let mut statistics = Stat {source_size: 0, target_size: 0};

    const SBVEC_SENTINEL_LEN: usize = LZ_MATCH_MAX_LEN * 2;
    const SBVEC_PREMATCH_LEN: usize = LZ_PREMATCH_SIZE;
    const SBVEC_POSTMATCH_LEN: usize = LZ_BLOCK_SIZE - SBVEC_PREMATCH_LEN;

    let sbvec = &mut vec![0u8; LZ_BLOCK_SIZE + SBVEC_SENTINEL_LEN][.. LZ_BLOCK_SIZE];
    let tbvec = &mut vec![0u8; SBVEC_PREMATCH_LEN * 3];

    loop {
        let sbvec_read_size = {
            let mut total_read_size = 0usize;
            while SBVEC_PREMATCH_LEN + total_read_size < sbvec.len() {
                let read_size = source.read(&mut sbvec[SBVEC_PREMATCH_LEN + total_read_size ..])?;
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
                lzenc.encode(cfg, &sbvec[ .. SBVEC_PREMATCH_LEN + sbvec_read_size], tbvec, spos)
            };
            target.write_u32::<byteorder::BE>(t as u32)?;
            statistics.target_size += 4;

            target.write_all(&tbvec[ .. t])?;
            spos = s;
            tpos += t;
        }
        statistics.source_size += spos as u64 - SBVEC_PREMATCH_LEN as u64;
        statistics.target_size += tpos as u64;
        let duration = std::time::Instant::now().duration_since(start_time);
        let duration_secs = duration.as_secs() as f64 + duration.subsec_nanos() as f64 * 1e-9;
        let mbps = statistics.source_size as f64 * 1e-6 / duration_secs;

        elog!(is_silent, "encode: {} bytes => {} bytes, {:.3}MB/s", spos - SBVEC_PREMATCH_LEN, tpos, mbps);
        unsafe {
            std::ptr::copy(
                sbvec.as_ptr().add(SBVEC_POSTMATCH_LEN),
                sbvec.as_mut_ptr(),
                SBVEC_PREMATCH_LEN);
        }
        lzenc.forward(SBVEC_POSTMATCH_LEN); // reset orz_lz encoder
    }

    // write a empty chunk to mark eof
    target.write_u32::<byteorder::BE>(0u32)?;
    statistics.target_size += 4;
    return Ok(statistics);
}

fn decode(
    is_silent: bool,
    target: &mut dyn std::io::Read,
    source: &mut dyn std::io::Write,
) -> std::io::Result<Stat> {

    let start_time = std::time::Instant::now();
    let mut lzdec = LZDecoder::new();
    let mut statistics = Stat {source_size: 0, target_size: 0};

    const SBVEC_SENTINEL_LEN: usize = LZ_MATCH_MAX_LEN * 2;
    const SBVEC_PREMATCH_LEN: usize = LZ_PREMATCH_SIZE;
    const SBVEC_POSTMATCH_LEN: usize = LZ_BLOCK_SIZE - SBVEC_PREMATCH_LEN;

    let sbvec = &mut vec![0u8; LZ_BLOCK_SIZE + SBVEC_SENTINEL_LEN][.. LZ_BLOCK_SIZE];
    let tbvec = &mut vec![0u8; SBVEC_PREMATCH_LEN * 3];

    let mut spos = SBVEC_PREMATCH_LEN;
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
            tpos += t;
        }

        if spos >= LZ_BLOCK_SIZE || t == 0 {
            statistics.source_size += spos as u64 - SBVEC_PREMATCH_LEN as u64;
            statistics.target_size += tpos as u64;

            let duration = std::time::Instant::now().duration_since(start_time);
            let duration_secs = duration.as_secs() as f64 + duration.subsec_nanos() as f64 * 1e-9;
            let mbps = statistics.source_size as f64 * 1e-6 / duration_secs;

            elog!(is_silent, "decode: {} bytes <= {} bytes, {:.3}MB/s", spos - SBVEC_PREMATCH_LEN, tpos, mbps);
            if t == 0 {
                break;
            }
            spos = SBVEC_PREMATCH_LEN;
            tpos = 0;
            unsafe {
                std::ptr::copy(
                    sbvec.as_ptr().offset(SBVEC_POSTMATCH_LEN as isize),
                    sbvec.as_mut_ptr(),
                    SBVEC_PREMATCH_LEN);
            }
            lzdec.forward(SBVEC_POSTMATCH_LEN);
        }
    }
    return Ok(statistics);
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    #[derive(StructOpt, Debug)]
    #[structopt(name = "orz", about = "an optimized ROLZ data compressor")]
    enum Opt {
        #[structopt(name = "encode", about = "Encode")]
        Encode {
            #[structopt(long = "silent", short = "s")] /// Run silently
            silent: bool,
            #[structopt(long = "level", short = "l", default_value = "3")] /// Set compression level (0..3)
            level: u8,
            #[structopt(parse(from_os_str))] /// Source file name, default to stdin
            ipath: Option<std::path::PathBuf>,
            #[structopt(parse(from_os_str))] /// Target file name, default to stdout
            opath: Option<std::path::PathBuf>,
        },

        #[structopt(name = "decode", about = "Decode")]
        Decode {
            #[structopt(long="silent", short = "s")] /// Run silently
            silent: bool,
            #[structopt(parse(from_os_str))] /// Source file name, default to stdin
            ipath: Option<std::path::PathBuf>,
            #[structopt(parse(from_os_str))] /// Target file name, default to stdout
            opath: Option<std::path::PathBuf>,
        },
    }

    let start_time = std::time::Instant::now();
    let args = {
        use structopt::StructOpt;
        Opt::from_args()
    };
    let is_silent = match args {
        Opt::Encode {silent, ..} | Opt::Decode {silent, ..} => silent,
    };

    let get_ifile = |ipath| -> Result<Box<dyn std::io::Read>, Box<dyn std::error::Error>> {
        return Ok(match ipath {
            &Some(ref p) => Box::new(std::fs::File::open(p)?),
            &None => Box::new(std::io::stdin()),
        });
    };
    let get_ofile = |opath| -> Result<Box<dyn std::io::Write>, Box<dyn std::error::Error>> {
        return Ok(match opath {
            &Some(ref p) => Box::new(std::fs::File::create(p)?),
            &None => Box::new(std::io::stdout()),
        });
    };

    // encode/decode
    let statistics = match args {
        Opt::Encode {level, ref ipath, ref opath, ..} => {
            encode(is_silent,
                &mut get_ifile(ipath)?,
                &mut get_ofile(opath)?,
                &match level {
                    0 => LZCfg {match_depth:  3, lazy_match_depth1:  2, lazy_match_depth2:  2},
                    1 => LZCfg {match_depth:  8, lazy_match_depth1:  6, lazy_match_depth2:  5},
                    2 => LZCfg {match_depth: 21, lazy_match_depth1: 17, lazy_match_depth2: 13},
                    3 => LZCfg {match_depth: 55, lazy_match_depth1: 44, lazy_match_depth2: 34},
                    _ => Err(format!("invalid level: {}", level))?,
                },
            ).or_else(|e| Err(format!("encoding failed: {}", e)))?
        }

        Opt::Decode {ref ipath, ref opath, ..} => {
            decode(is_silent,
                &mut get_ifile(ipath)?,
                &mut get_ofile(opath)?,
            ).or_else(|e| Err(format!("decoding failed: {}", e)))?
        }
    };

    // dump statistics
    let duration = std::time::Instant::now().duration_since(start_time);
    let duration_secs = duration.as_secs() as f64 + duration.subsec_nanos() as f64 * 1e-9;
    elog!(is_silent, "statistics:");
    elog!(is_silent, "  size:  {0} bytes {2} {1} bytes",
        statistics.source_size,
        statistics.target_size,
        match args {
            Opt::Encode {..} => "=>",
            Opt::Decode {..} => "<=",
        });
    elog!(is_silent, "  ratio: {:.2}%", statistics.target_size as f64 * 100.0 / statistics.source_size as f64);
    elog!(is_silent, "  time:  {:.3} sec", duration_secs);
    return Ok(());
}
