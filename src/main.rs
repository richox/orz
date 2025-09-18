use std::{
    error::Error,
    fs::File,
    io::{Read, Write, stdin, stdout},
    path::PathBuf,
};

use clap::Parser;
use log::LevelFilter;
use orz::{decode, encode, lz::LZCfg};
use simplelog::{CombinedLogger, ConfigBuilder, LevelPadding, TermLogger, TerminalMode};

fn main() -> Result<(), Box<dyn Error>> {
    #[derive(Parser, Debug)]
    #[command(name = "orz", about = "an optimized ROLZ data compressor")]
    enum Opt {
        #[command(name = "encode", about = "Encode")]
        Encode {
            #[arg(long = "silent", short = 's')]
            /// Run silently
            silent: bool,
            #[arg(long = "level", short = 'l', default_value = "2")]
            /// Set compression level (0..2)
            level: u8,
            #[arg()]
            /// Source file name, default to stdin
            ipath: Option<PathBuf>,
            #[arg()]
            /// Target file name, default to stdout
            opath: Option<PathBuf>,
        },

        #[command(name = "decode", about = "Decode")]
        Decode {
            #[arg(long = "silent", short = 's')]
            /// Run silently
            silent: bool,
            #[arg()]
            /// Source file name, default to stdin
            ipath: Option<PathBuf>,
            #[arg()]
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
                    0 => LZCfg::new(5, 3, 2),
                    1 => LZCfg::new(15, 9, 6),
                    2 => LZCfg::new(45, 27, 18),
                    _ => return Err(format!("invalid level: {}", level).into()),
                },
            )
            .map_err(|e| format!("encoding failed: {}", e))?
            .log_finish(true);
        }
        Opt::Decode { ipath, opath, .. } => {
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
