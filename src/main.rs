extern crate byteorder;
extern crate crc32c_hw;
extern crate log;
extern crate simplelog;
extern crate structopt;
extern crate unchecked_index;

use orz::{encode, decode};
use orz::lz::LZCfg;
use structopt::StructOpt;

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
    let args = Opt::from_args();

    // init logger
    simplelog::CombinedLogger::init(match args {
        Opt::Encode {silent:  true, ..} | Opt::Decode {silent:  true, ..} => vec![],
        Opt::Encode {silent: false, ..} | Opt::Decode {silent: false, ..} => vec![{
            let config = simplelog::ConfigBuilder::new()
                .set_time_level(simplelog::LevelFilter::Off)
                .set_location_level(simplelog::LevelFilter::Off)
                .set_target_level(simplelog::LevelFilter::Off)
                .set_thread_level(simplelog::LevelFilter::Off)
                .set_level_padding(simplelog::LevelPadding::Off)
                .build();
            simplelog::TermLogger::new(simplelog::LevelFilter::max(), config, simplelog::TerminalMode::Stderr).unwrap()
        }],
    })?;

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
            encode(&mut get_ifile(ipath)?, &mut get_ofile(opath)?, &match level {
                0 => LZCfg {match_depth:  5, lazy_match_depth1:  3, lazy_match_depth2:  2},
                1 => LZCfg {match_depth: 15, lazy_match_depth1:  9, lazy_match_depth2:  6},
                2 => LZCfg {match_depth: 45, lazy_match_depth1: 27, lazy_match_depth2: 18},
                _ => Err(format!("invalid level: {}", level))?,
            }).or_else(|e| Err(format!("encoding failed: {}", e)))?
        }

        Opt::Decode {ref ipath, ref opath, ..} => {
            decode(&mut get_ifile(ipath)?, &mut get_ofile(opath)?).or_else(|e| Err(format!("decoding failed: {}", e)))?
        }
    };

    // dump statistics
    let duration = std::time::Instant::now().duration_since(start_time);
    let duration_secs = duration.as_secs() as f64 + duration.subsec_nanos() as f64 * 1e-9;
    log::info!("statistics:");
    log::info!("  size:  {0} bytes {2} {1} bytes",
        statistics.source_size,
        statistics.target_size,
        match args {
            Opt::Encode {..} => "=>",
            Opt::Decode {..} => "<=",
        });
    log::info!("  ratio: {:.2}%", statistics.target_size as f64 * 100.0 / statistics.source_size as f64);
    log::info!("  time:  {:.3} sec", duration_secs);
    return Ok(());
}
