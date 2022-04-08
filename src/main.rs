use log::LevelFilter;
use simplelog::CombinedLogger;
use simplelog::ConfigBuilder;
use simplelog::LevelPadding;
use simplelog::SharedLogger;
use simplelog::TermLogger;
use simplelog::TerminalMode;
use std::error::Error;
use std::fs::File;
use std::io::stdin;
use std::io::stdout;
use std::io::Read;
use std::io::Write;
use std::path::PathBuf;
use std::time::Instant;

use orz::lz::LZCfg;
use orz::{decode, encode};
use structopt::StructOpt;

fn main() -> Result<(), Box<dyn Error>> {
    #[derive(StructOpt, Debug)]
    #[structopt(name = "orz", about = "an optimized ROLZ data compressor")]
    enum Opt {
        #[structopt(name = "encode", about = "Encode")]
        Encode {
            #[structopt(long = "silent", short = "s")]
            /// Run silently
            silent: bool,
            #[structopt(long = "level", short = "l", default_value = "2")]
            /// Set compression level (0..2)
            level: u8,
            #[structopt(parse(from_os_str))]
            /// Source file name, default to stdin
            ipath: Option<PathBuf>,
            #[structopt(parse(from_os_str))]
            /// Target file name, default to stdout
            opath: Option<PathBuf>,
        },

        #[structopt(name = "decode", about = "Decode")]
        Decode {
            #[structopt(long = "silent", short = "s")]
            /// Run silently
            silent: bool,
            #[structopt(parse(from_os_str))]
            /// Source file name, default to stdin
            ipath: Option<PathBuf>,
            #[structopt(parse(from_os_str))]
            /// Target file name, default to stdout
            opath: Option<PathBuf>,
        },
    }

    let start_time = std::time::Instant::now();
    let args = Opt::from_args();

    // init logger
    CombinedLogger::init(match args {
        Opt::Encode { silent: true, .. } | Opt::Decode { silent: true, .. } => vec![],
        Opt::Encode { silent: false, .. } | Opt::Decode { silent: false, .. } => vec![{
            let config = ConfigBuilder::new()
                .set_time_level(LevelFilter::Off)
                .set_location_level(LevelFilter::Off)
                .set_target_level(LevelFilter::Off)
                .set_thread_level(LevelFilter::Off)
                .set_level_padding(LevelPadding::Off)
                .build();
            TermLogger::new(LevelFilter::max(), config, TerminalMode::Stderr).unwrap()
                as Box<dyn SharedLogger>
        }],
    })?;

    let get_ifile = |ipath| {
        Result::<_, Box<dyn Error>>::Ok(match ipath {
            Some(p) => Box::new(File::open(p)?) as Box<dyn Read>,
            None => Box::new(stdin()),
        })
    };
    let get_ofile = |opath| {
        Result::<_, Box<dyn Error>>::Ok(match opath {
            Some(p) => Box::new(File::create(p)?) as Box<dyn Write>,
            None => Box::new(stdout()),
        })
    };

    // encode/decode
    let statistics = match &args {
        Opt::Encode {
            level,
            ipath,
            opath,
            ..
        } => encode(
            &mut get_ifile(ipath.as_deref())?,
            &mut get_ofile(opath.as_deref())?,
            &match level {
                0 => LZCfg {
                    match_depth: 5,
                    lazy_match_depth1: 3,
                    lazy_match_depth2: 2,
                },
                1 => LZCfg {
                    match_depth: 15,
                    lazy_match_depth1: 9,
                    lazy_match_depth2: 6,
                },
                2 => LZCfg {
                    match_depth: 45,
                    lazy_match_depth1: 27,
                    lazy_match_depth2: 18,
                },
                _ => return Err(format!("invalid level: {}", level).into()),
            },
        )
        .map_err(|e| format!("encoding failed: {}", e))?,

        Opt::Decode {
            ref ipath,
            ref opath,
            ..
        } => decode(
            &mut get_ifile(ipath.as_deref())?,
            &mut get_ofile(opath.as_deref())?,
        )
        .map_err(|e| format!("decoding failed: {}", e))?,
    };

    // dump statistics
    let duration = Instant::now().duration_since(start_time);
    let duration_secs = duration.as_secs() as f64 + duration.subsec_nanos() as f64 * 1e-9;
    log::info!("statistics:");
    log::info!(
        "  size:  {0} bytes {2} {1} bytes",
        statistics.source_size,
        statistics.target_size,
        match &args {
            Opt::Encode { .. } => "=>",
            Opt::Decode { .. } => "<=",
        }
    );
    log::info!(
        "  ratio: {:.2}%",
        statistics.target_size as f64 * 100.0 / statistics.source_size as f64
    );
    log::info!("  time:  {:.3} sec", duration_secs);
    Ok(())
}
