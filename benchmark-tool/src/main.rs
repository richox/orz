extern crate checksums;
extern crate libc;
extern crate madato;
extern crate num_format;
extern crate subprocess;
extern crate tempfile;

use num_format::ToFormattedString;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = std::env::args().collect::<Vec<_>>();
    if args.len() != 2 {
        Err("usage: ./bench.rs <bench-file>")?;
    }

    let encoders = [
        ("**orz -l0**", vec!["orz", "encode", "-s", "-l0"], vec!["orz", "decode", "-s"]),
        ("**orz -l1**", vec!["orz", "encode", "-s", "-l1"], vec!["orz", "decode", "-s"]),
        ("**orz -l2**", vec!["orz", "encode", "-s", "-l2"], vec!["orz", "decode", "-s"]),
        ("gzip -6",     vec!["gzip", "-6"], vec!["gzip", "-d"]),
        ("bzip2 -9",    vec!["bzip2", "-9"], vec!["bzip2", "-d"]),
        ("xz -6",       vec!["xz", "-6"], vec!["xz", "-d"]),
        ("brotli -7",   vec!["brotli", "-7"], vec!["brotli", "-d"]),
        ("brotli -8",   vec!["brotli", "-8"], vec!["brotli", "-d"]),
        ("brotli -9",   vec!["brotli", "-9"], vec!["brotli", "-d"]),
        ("zstd -10",    vec!["zstd", "-10"], vec!["zstd", "-d"]),
        ("zstd -15",    vec!["zstd", "-15"], vec!["zstd", "-d"]),
        ("zstd -19",    vec!["zstd", "-19"], vec!["zstd", "-d"]),
        ("lzfse",       vec!["lzfse", "-encode"], vec!["lzfse", "-decode"]),
    ];
    let temp_dir = tempfile::tempdir()?;
    let bench_file_path = std::path::PathBuf::from(&args[1]);
    let mut rows = vec![];

    for (name, enc_command, dec_command) in &encoders {
        let (size, enc_time, dec_time) = bench(&temp_dir, &bench_file_path, name, enc_command, dec_command)?;
        eprintln!("size: {}, enc_time: {:.3}s, dec_time: {:.3}s", size, enc_time, dec_time);

        let mut row = madato::types::TableRow::new();
        row.insert("name".to_owned(), name.to_string());
        row.insert("compressed size".to_owned(), size.to_formatted_string(&num_format::Locale::en));
        row.insert("encode time".to_owned(), format!("{:.3}s", enc_time));
        row.insert("decode time".to_owned(), format!("{:.3}s", dec_time));
        rows.push(row);
    }
    rows.sort_by(|row1, row2| row1.get("compressed size").cmp(&row2.get("compressed size")));
    println!("{}", madato::mk_table(&rows[..], &None));
    Ok(())
}

fn bench(
    temp_dir: &tempfile::TempDir,
    bench_file_path: &std::path::Path,
    name: &str,
    enc_command: &[&str],
    dec_command: &[&str],
) -> Result<(u64, f64, f64), Box<dyn std::error::Error>> {
    eprintln!("start benchmarking {}...", name);
    let mut enc_times = vec![];
    let mut dec_times = vec![];

    for i in 0..3 {
        // encode
        {
            let bench_file = std::fs::File::open(bench_file_path)?;
            let bench_enc_output_file = std::fs::File::create(temp_dir.path().join("enc_output"))?;
            let t = get_children_process_utime_sec();
            if !subprocess::Popen::create(enc_command, subprocess::PopenConfig {
                stdin:  subprocess::Redirection::File(bench_file),
                stdout: subprocess::Redirection::File(bench_enc_output_file),
                stderr: subprocess::Redirection::None,
                .. Default::default()
            })?.wait()?.success() {
                Err(format!("{}.encode: exit status not success", name))?;
            }
            enc_times.push(get_children_process_utime_sec() - t);
            eprintln!(" => round {}: finished encoding: time={:.3}s", i, enc_times.last().unwrap());
        }

        // decode
        {
            let bench_enc_output_file = std::fs::File::open(temp_dir.path().join("enc_output"))?;
            let bench_dec_output_file = std::fs::File::create(temp_dir.path().join("dec_output"))?;
            let t = get_children_process_utime_sec();
            if !subprocess::Popen::create(dec_command, subprocess::PopenConfig {
                stdin:  subprocess::Redirection::File(bench_enc_output_file),
                stdout: subprocess::Redirection::File(bench_dec_output_file),
                stderr: subprocess::Redirection::None,
                .. Default::default()
            })?.wait()?.success() {
                Err(format!("{}.decode: exit status not success", name))?;
            }
            dec_times.push(get_children_process_utime_sec() - t);
            eprintln!(" => round {}: finished decoding: time={:.3}s", i, dec_times.last().unwrap());
        }

        // verify
        let hash1 = checksums::hash_file(&bench_file_path, checksums::Algorithm::MD5);
        let hash2 = checksums::hash_file(&temp_dir.path().join("dec_output"), checksums::Algorithm::MD5);
        if hash1 != hash2 {
            Err(format!("{}.decode: wrong result", name))?;
        }
    }
    let size = std::fs::metadata(temp_dir.path().join("enc_output"))?.len();
    let enc_times_min = enc_times.into_iter().min_by(|a, b| a.partial_cmp(b).unwrap()).unwrap();
    let dec_times_min = dec_times.into_iter().min_by(|a, b| a.partial_cmp(b).unwrap()).unwrap();
    Ok((size, enc_times_min, dec_times_min))
}

fn get_children_process_utime_sec() -> f64 {
    let mut rusage = unsafe {std::mem::zeroed()};
    unsafe {
        libc::getrusage(libc::RUSAGE_CHILDREN, &mut rusage);
    }
    return rusage.ru_utime.tv_sec as f64 + rusage.ru_utime.tv_usec as f64 * 1e-6;
}
