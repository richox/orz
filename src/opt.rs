use either::Either;
use libc;
use std::fs::File;
use std::io::{self, Error, ErrorKind, Result, Stdin, Stdout};
use std::path::{Path, PathBuf};
use structopt;
use orz::LZCfg;

pub type IStream = Either<File, Stdin>;
pub type OStream = Either<File, Stdout>;

#[derive(StructOpt, Debug)]
#[structopt(name = "orz", about = "an optimized ROLZ data compressor")]
#[structopt(raw(setting = "structopt::clap::AppSettings::ColoredHelp"))]
pub enum Opt {
    #[structopt(name = "encode", about = "Encode")]
    Encode(Encode),
    #[structopt(name = "decode", about = "Decode")]
    Decode(Decode),
}

#[derive(StructOpt, Debug)]
#[structopt(raw(setting = "structopt::clap::AppSettings::ColoredHelp"))]
pub struct Encode {
    #[structopt(short = "b", default_value = "16777216", help = "Set compression block size")]
    block_size: usize,

    #[structopt(short = "l", default_value = "4", help = "Set compression level (0..4)")]
    level: u8,

    #[structopt(help = "Input filename, default to stdin", parse(from_os_str))]
    ipath: Option<PathBuf>,

    #[structopt(help = "Output filename, default to stdout", parse(from_os_str))]
    opath: Option<PathBuf>,
}

#[derive(StructOpt, Debug)]
#[structopt(raw(setting = "structopt::clap::AppSettings::ColoredHelp"))]
pub struct Decode {
    #[structopt(help = "Input filename, default to stdin", parse(from_os_str))]
    ipath: Option<PathBuf>,

    #[structopt(help = "Output filename, default to stdout", parse(from_os_str))]
    opath: Option<PathBuf>,
}

impl Encode {
    pub fn get_ifile(&self) -> Result<IStream> {
        get_ifile(self.ipath.as_ref())
    }

    pub fn get_ofile(&self) -> Result<OStream> {
        get_ofile(self.opath.as_ref())
    }

    pub fn config(&self) -> Result<LZCfg> {
        Ok(match self.level {
            0 => LZCfg {
                block_size: self.block_size,
                match_depth: 2,
                lazy_match_depth1: 1,
                lazy_match_depth2: 1,
            },
            1 => LZCfg {
                block_size: self.block_size,
                match_depth: 4,
                lazy_match_depth1: 1,
                lazy_match_depth2: 1,
            },
            2 => LZCfg {
                block_size: self.block_size,
                match_depth: 8,
                lazy_match_depth1: 2,
                lazy_match_depth2: 1,
            },
            3 => LZCfg {
                block_size: self.block_size,
                match_depth: 16,
                lazy_match_depth1: 4,
                lazy_match_depth2: 2,
            },
            4 => LZCfg {
                block_size: self.block_size,
                match_depth: 32,
                lazy_match_depth1: 8,
                lazy_match_depth2: 4,
            },
            level => {
                return Err(Error::new(
                    ErrorKind::Other,
                    format!("invalid level: {}", level),
                ))
            }
        })
    }
}

impl Decode {
    pub fn get_ifile(&self) -> Result<IStream> {
        get_ifile(self.ipath.as_ref())
    }

    pub fn get_ofile(&self) -> Result<OStream> {
        get_ofile(self.opath.as_ref())
    }
}

fn get_ifile<T: AsRef<Path>>(path: Option<T>) -> Result<IStream> {
    match path {
        None => {
            if unsafe { libc::isatty(1) } != 0 {
                return Err(Error::new(
                    ErrorKind::Other,
                    "Compressed data cannot be read from terminal",
                ));
            }

            Ok(Either::Right(io::stdin()))
        }
        Some(ref path) => File::open(path).map(Either::Left),
    }
}

fn get_ofile<T: AsRef<Path>>(path: Option<T>) -> Result<OStream> {
    match path {
        None => {
            if unsafe { libc::isatty(2) } != 0 {
                return Err(Error::new(
                    ErrorKind::Other,
                    "Compressed data cannot be written to terminal",
                ));
            }

            Ok(Either::Right(io::stdout()))
        }
        Some(ref path) => File::create(path).map(Either::Left),
    }
}
