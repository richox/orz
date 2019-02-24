#[macro_use] extern crate structopt;
extern crate byteorder;

mod _constants;
mod bits;
mod aux;
mod huffman;
mod lz;
mod matchfinder;
mod mtf;

use byteorder::ReadBytesExt;
use byteorder::WriteBytesExt;
use self::lz::LZCfg;
use self::lz::LZDecoder;
use self::lz::LZEncoder;

use _constants::lz_roid_array::LZ_ROID_SIZE;
use _constants::lz_roid_array::LZ_MF_BUCKET_ITEM_SIZE;

const LZ_BLOCK_SIZE: usize = 33554432;
const LZ_PREMATCH_SIZE: usize = LZ_BLOCK_SIZE / 2;
const LZ_CHUNK_SIZE: usize = 393216;
const LZ_MATCH_MAX_LEN: usize = 255;
const LZ_MATCH_MIN_LEN: usize = 4;
const LZ_MF_BUCKET_ITEM_HASH_SIZE: usize = 8192;

struct Stat {
    pub source_size: u64,
    pub target_size: u64,
}

fn encode(source: &mut std::io::Read, target: &mut std::io::Write, cfg: &LZCfg) -> std::io::Result<Stat> {
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
            tpos = tpos + t;
        }
        statistics.source_size += spos as u64 - SBVEC_PREMATCH_LEN as u64;
        statistics.target_size += tpos as u64;
        let duration = std::time::Instant::now().duration_since(start_time);
        let duration_secs = duration.as_secs() as f64 + duration.subsec_nanos() as f64 * 1e-9;
        let mbps = statistics.source_size as f64 * 1e-6 / duration_secs;

        eprintln!("encode: {} bytes => {} bytes, {:.3}MB/s", spos - SBVEC_PREMATCH_LEN, tpos, mbps);
        unsafe {
            std::ptr::copy(
                sbvec.as_ptr().offset(SBVEC_POSTMATCH_LEN as isize),
                sbvec.as_mut_ptr(),
                SBVEC_PREMATCH_LEN);
        }
        lzenc.forward(SBVEC_POSTMATCH_LEN as u32); // reset orz_lz encoder
    }

    // write a empty chunk to mark eof
    target.write_u32::<byteorder::BE>(0u32)?;
    statistics.target_size += 4;
    Ok(statistics)
}

fn decode(target: &mut std::io::Read, source: &mut std::io::Write) -> std::io::Result<Stat> {
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
            tpos = t + tpos;
        }

        if spos >= LZ_BLOCK_SIZE || t == 0 {
            statistics.source_size += spos as u64 - SBVEC_PREMATCH_LEN as u64;
            statistics.target_size += tpos as u64;

            let duration = std::time::Instant::now().duration_since(start_time);
            let duration_secs = duration.as_secs() as f64 + duration.subsec_nanos() as f64 * 1e-9;
            let mbps = statistics.source_size as f64 * 1e-6 / duration_secs;

            eprintln!("decode: {} bytes <= {} bytes, {:.3}MB/s", spos - SBVEC_PREMATCH_LEN, tpos, mbps);
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
            lzdec.forward(SBVEC_POSTMATCH_LEN as u32);
        }
    }
    Ok(statistics)
}

fn main() -> Result<(), Box<std::error::Error>> {
    #[derive(StructOpt, Debug)]
    #[structopt(name = "orz", about = "an optimized ROLZ data compressor")]
    enum Opt {
        #[structopt(name = "encode", about = "Encode")]
        Encode {
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

    let start_time = std::time::Instant::now();
    let args = {
        use structopt::StructOpt;
        Opt::from_args()
    };

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
        &Opt::Encode {level, ref ipath, ref opath} => {
            encode(
                &mut get_ifile(ipath)?,
                &mut get_ofile(opath)?,
                &match level {
                    0 => LZCfg {match_depth:  3, lazy_match_depth1: 1, lazy_match_depth2: 1, lazy_match_depth3: 1},
                    1 => LZCfg {match_depth:  5, lazy_match_depth1: 2, lazy_match_depth2: 1, lazy_match_depth3: 1},
                    2 => LZCfg {match_depth:  8, lazy_match_depth1: 3, lazy_match_depth2: 2, lazy_match_depth3: 1},
                    3 => LZCfg {match_depth: 13, lazy_match_depth1: 5, lazy_match_depth2: 3, lazy_match_depth3: 2},
                    4 => LZCfg {match_depth: 21, lazy_match_depth1: 8, lazy_match_depth2: 5, lazy_match_depth3: 3},
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
    let duration = std::time::Instant::now().duration_since(start_time);
    let duration_secs = duration.as_secs() as f64 + duration.subsec_nanos() as f64 * 1e-9;
    eprintln!("statistics:");
    eprintln!("  size:  {0} bytes {2} {1} bytes",
        statistics.source_size,
        statistics.target_size,
        match &args {
            &Opt::Encode {..} => "=>",
            &Opt::Decode {..} => "<=",
        });
    eprintln!("  ratio: {:.2}%", statistics.target_size as f64 * 100.0 / statistics.source_size as f64);
    eprintln!("  time:  {:.3} sec", duration_secs);
    Ok(())
}
