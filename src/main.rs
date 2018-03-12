extern crate orz;
#[macro_use] extern crate structopt;

use orz::*;
use structopt::*;

#[derive(StructOpt, Debug)]
#[structopt(name = "orz", about = "an optimized ROLZ data compressor")]
enum Opt {
    #[structopt(name = "encode", about = "Encode")]
    Encode {
        #[structopt(short = "b", default_value = "16777216", help = "Set compression block size")]
        block_size: usize,
        #[structopt(short = "l", default_value = "4", help = "Set compression level (0..4)")]
        level: u8,
        #[structopt(help = "Input filename, default to stdin", parse(from_os_str))]
        ipath: Option<std::path::PathBuf>,
        #[structopt(help = "Output filename, default to stdout", parse(from_os_str))]
        opath: Option<std::path::PathBuf>,
    },

    #[structopt(name = "decode", about = "Decode")]
    Decode {
        #[structopt(help = "Input filename, default to stdin", parse(from_os_str))]
        ipath: Option<std::path::PathBuf>,
        #[structopt(help = "Output filename, default to stdout", parse(from_os_str))]
        opath: Option<std::path::PathBuf>,
    },
}

fn main_wrapped() -> Result<(), String> {
    let args = Opt::from_args();

    let get_ifile = |ipath| -> Result<Box<std::io::Read>, String> {
        Ok(match ipath {
            &None        => Box::new(std::io::stdin()),
            &Some(ref p) => Box::new(
                std::fs::File::open(p).or_else(|e| Err(format!("cannot open input file: {}", e)))?),
        })
    };
    let get_ofile = |opath| -> Result<Box<std::io::Write>, String> {
        Ok(match opath {
            &None        => Box::new(std::io::stdout()),
            &Some(ref p) => Box::new(
                std::fs::File::create(p).or_else(|e| Err(format!("cannot open output file: {}", e)))?),
        })
    };

    // encode/decode
    let time_start = std::time::SystemTime::now();
    let statistics = match &args {
        &Opt::Encode {block_size, level, ref ipath, ref opath} => {
            Orz::encode(
                &mut get_ifile(ipath)?,
                &mut get_ofile(opath)?,
                &match level {
                    0 => LZCfg {block_size: block_size, match_depth:  2, lazy_match_depth1: 1, lazy_match_depth2: 1},
                    1 => LZCfg {block_size: block_size, match_depth:  3, lazy_match_depth1: 2, lazy_match_depth2: 1},
                    2 => LZCfg {block_size: block_size, match_depth:  5, lazy_match_depth1: 3, lazy_match_depth2: 2},
                    3 => LZCfg {block_size: block_size, match_depth:  8, lazy_match_depth1: 5, lazy_match_depth2: 3},
                    4 => LZCfg {block_size: block_size, match_depth: 13, lazy_match_depth1: 8, lazy_match_depth2: 5},
                    _ => Err(format!("invalid level: {}", level))?,
                },
            ).or_else(|e| Err(format!("encoding failed: {}", e)))?
        }

        &Opt::Decode {ref ipath, ref opath} => {
            Orz::decode(
                &mut get_ifile(ipath)?,
                &mut get_ofile(opath)?,
            ).or_else(|e| Err(format!("decoding failed: {}", e)))?
        }
    };
    let time_end = std::time::SystemTime::now();

    // dump statistics
    eprintln!("statistics:");
    eprintln!("  size:  {0} bytes {2} {1} bytes",
        statistics.source_size,
        statistics.target_size,
        match &args {
            &Opt::Encode {..} => "=>",
            &Opt::Decode {..} => "<=",
        });
    eprintln!("  ratio: {:.2}%",
        statistics.target_size as f64 * 100.0 / statistics.source_size as f64);
    eprintln!("  time:  {:.3} sec",
        time_end.duration_since(time_start).unwrap().as_secs() as f64 +
        time_end.duration_since(time_start).unwrap().subsec_nanos() as f64 * 1e-9);
    Ok(())
}

fn main() {
    if let Err(e) = main_wrapped() {
        eprintln!("error: {}", e);
        std::process::exit(-1);
    }
}
