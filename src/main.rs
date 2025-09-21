use std::{
    error::Error,
    fs::File,
    io::{Read, Write, stdin, stdout},
    path::PathBuf,
};

use clap::Parser;
use orz::{
    CountRead, CountWrite, LZCfg, ProgressLogger, SilentProgressLogger, SimpleProgressLogger,
    decode, encode,
};

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

    impl Opt {
        fn is_silent(&self) -> bool {
            match self {
                Opt::Encode { silent, .. } => *silent,
                Opt::Decode { silent, .. } => *silent,
            }
        }
    }

    let args = Opt::parse();

    // init progress logger
    let mut progress_logger: Box<dyn ProgressLogger> = if args.is_silent() {
        Box::new(SilentProgressLogger)
    } else {
        Box::new(SimpleProgressLogger::new())
    };

    // init input/output
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
                &mut CountRead::new(get_ifile(ipath.as_deref())?),
                &mut CountWrite::new(get_ofile(opath.as_deref())?),
                &match level {
                    0 => LZCfg::new(5, 3, 2),
                    1 => LZCfg::new(15, 9, 6),
                    2 => LZCfg::new(45, 27, 18),
                    _ => return Err(format!("invalid level: {}", level).into()),
                },
                &mut progress_logger,
            )
            .map_err(|e| format!("encoding failed: {}", e))?;
        }
        Opt::Decode { ipath, opath, .. } => {
            decode(
                &mut CountRead::new(get_ifile(ipath.as_deref())?),
                &mut CountWrite::new(get_ofile(opath.as_deref())?),
                &mut progress_logger,
            )
            .map_err(|e| format!("decoding failed: {}", e))?;
        }
    };
    Ok(())
}
