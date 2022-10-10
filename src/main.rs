use log::LevelFilter;
use simplelog::CombinedLogger;
use simplelog::ConfigBuilder;
use simplelog::LevelPadding;
use simplelog::TermLogger;
use simplelog::TerminalMode;
use std::error::Error;
use std::fs::File;
use std::io::stdin;
use std::io::stdout;
use std::io::Read;
use std::io::Write;
use std::path::PathBuf;

use clap::Parser;
use orz::lz::LZCfg;
use orz::{decode, encode};

fn main() -> Result<(), Box<dyn Error>> {
    #[derive(Parser, Debug)]
    #[clap(name = "orz", about = "an optimized ROLZ data compressor")]
    enum Opt {
        #[clap(name = "encode", about = "Encode")]
        Encode {
            #[clap(long = "silent", short = 's')]
            /// Run silently
            silent: bool,
            #[clap(long = "level", short = 'l', default_value = "2")]
            /// Set compression level (0..2)
            level: u8,
            #[clap(parse(from_os_str))]
            /// Source file name, default to stdin
            ipath: Option<PathBuf>,
            #[clap(parse(from_os_str))]
            /// Target file name, default to stdout
            opath: Option<PathBuf>,
        },

        #[clap(name = "decode", about = "Decode")]
        Decode {
            #[clap(long = "silent", short = 's')]
            /// Run silently
            silent: bool,
            #[clap(parse(from_os_str))]
            /// Source file name, default to stdin
            ipath: Option<PathBuf>,
            #[clap(parse(from_os_str))]
            /// Target file name, default to stdout
            opath: Option<PathBuf>,
        },
    }
    let args = Opt::parse();

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
            TermLogger::new(
                LevelFilter::max(),
                config,
                TerminalMode::Stderr,
                simplelog::ColorChoice::Auto,
            )
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
    match &args {
        Opt::Encode {
            level,
            ipath,
            opath,
            ..
        } => {
            encode(
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
            .map_err(|e| format!("encoding failed: {}", e))?
            .log_finish(true);
        }
        Opt::Decode {
            ref ipath,
            ref opath,
            ..
        } => {
            decode(
                &mut get_ifile(ipath.as_deref())?,
                &mut get_ofile(opath.as_deref())?,
            )
            .map_err(|e| format!("decoding failed: {}", e))?
            .log_finish(true);
        }
    };
    Ok(())
}
