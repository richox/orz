#[macro_use]
extern crate structopt;

extern crate either;
extern crate libc;
extern crate orz;

mod opt;

use std::io::Result;

fn main_wrapped() -> Result<()> {
    use opt::Opt;
    use orz::*;
    use structopt::StructOpt;
    use std::time::Instant;

    let opt = Opt::from_args();

    let time_start = Instant::now();

    let statistics = match opt {
        Opt::Encode(ref encode) => {
            let mut ifile = encode.get_ifile()?;
            let mut ofile = encode.get_ofile()?;
            Orz::encode(&mut ifile, &mut ofile, &encode.config()?)?
        }

        Opt::Decode(ref decode) => {
            let mut ifile = decode.get_ifile()?;
            let mut ofile = decode.get_ofile()?;
            Orz::decode(&mut ifile, &mut ofile)?
        }
    };

    let time_end = Instant::now();

    // dump statistics
    eprintln!("statistics:");
    eprintln!(
        "  size:  {0} bytes {2} {1} bytes",
        statistics.source_size,
        statistics.target_size,
        match opt {
            Opt::Encode { .. } => "=>",
            Opt::Decode { .. } => "<=",
        }
    );
    eprintln!(
        "  ratio: {:.2}%",
        statistics.target_size as f64 * 100.0 / statistics.source_size as f64
    );
    eprintln!(
        "  time:  {:.3} sec",
        time_end.duration_since(time_start).as_secs() as f64
            + time_end.duration_since(time_start).subsec_nanos() as f64 * 1e-9
    );
    Ok(())
}

fn main() {
    if let Err(e) = main_wrapped() {
        eprintln!("error: {}", e);
        std::process::exit(-1);
    }
}
