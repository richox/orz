mod orz;

pub struct Argument {
    pub mode: Mode,
    pub ifile: std::fs::File,
    pub ofile: std::fs::File,
}

pub enum Mode {
    Encode(orz::Params),
    Decode,
}

pub fn parse_args() -> Result<Argument, String> {
    eprintln!("orz: an optimized ROLZ data-compressor");
    eprintln!("usage: ");
    eprintln!("  encode: orz e[0..4] <input-file> <output-file>");
    eprintln!("  decode: orz d       <input-file> <output-file>");
    eprintln!("");

    let mut args = std::env::args().skip(1);
    let args_mode = args.next().ok_or("mode not specified")?;
    let args_source_file = args.next().ok_or("source file name not specified")?;
    let args_target_file = args.next().ok_or("target file name not specified")?;

    return Ok(Argument {
        mode: match args_mode.as_str() {
            "e0" => Mode::Encode(
               orz::Params {match_depth:  2, match_depth_lazy_evaluation1: 1, match_depth_lazy_evaluation2: 1}),
            "e1" => Mode::Encode(
               orz::Params {match_depth:  3, match_depth_lazy_evaluation1: 2, match_depth_lazy_evaluation2: 1}),
            "e2" => Mode::Encode(
               orz::Params {match_depth:  5, match_depth_lazy_evaluation1: 3, match_depth_lazy_evaluation2: 2}),
            "e3" => Mode::Encode(
               orz::Params {match_depth:  8, match_depth_lazy_evaluation1: 5, match_depth_lazy_evaluation2: 3}),
            "e4" => Mode::Encode(
               orz::Params {match_depth: 13, match_depth_lazy_evaluation1: 8, match_depth_lazy_evaluation2: 5}),
            "d"  => Mode::Decode,
            invalid_mode @ _ => return Err(format!("invalid mode: {}", invalid_mode)),
        },
        ifile: std::fs::File::open(args_source_file).or_else(|e|
            Err(format!("cannot open input file: {}", e)))?,
        ofile: std::fs::File::create(args_target_file).or_else(|e|
            Err(format!("cannot open output file: {}", e)))?,
    });
}

fn main() {
    match || -> Result<(), String> {
        let mut args = parse_args()?;
        let time_start = std::time::SystemTime::now();
        let statistics = {
            let statistics = match args.mode {
                Mode::Encode(params) => orz::encode(
                    &mut std::io::BufReader::new(&mut args.ifile),
                    &mut std::io::BufWriter::new(&mut args.ofile), &params).or_else(|e|
                        Err(format!("encoding failed: {}", e))),
                Mode::Decode => orz::decode(
                    &mut std::io::BufReader::new(&mut args.ifile),
                    &mut std::io::BufWriter::new(&mut args.ofile)).or_else(|e|
                        Err(format!("decoding failed: {}", e))),
            }?;
            args.ifile.sync_all().or(Err("synchronizing source file failed"))?;
            args.ofile.sync_all().or(Err("synchronizing source file failed"))?;
            statistics
        };
        let time_end = std::time::SystemTime::now();
        let duration = time_end.duration_since(time_start).unwrap();
        eprintln!("statistics:");
        eprintln!("  size:  {} bytes <= {} bytes", statistics.source_size, statistics.target_size);
        eprintln!("  ratio: {:.2}%", statistics.target_size as f64 * 100.0 / statistics.source_size as f64);
        eprintln!("  time:  {:.3} sec", duration.as_secs() as f64 + duration.subsec_nanos() as f64 * 1e-9);
        return Ok(());
    }() {
        Ok(_) => (),
        Err(e) => {
            eprintln!("error: {}", e);
            std::process::exit(-1);
        }
    }
}
