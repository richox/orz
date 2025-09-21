use std::time::Instant;

pub trait ProgressLogger {
    fn set_is_encode(&mut self, is_encode: bool);
    fn log(&mut self, num_input_bytes: usize, num_output_bytes: usize);
    fn finish(&mut self, _num_input_bytes: usize, _num_output_bytes: usize);
}

pub struct SilentProgressLogger;

impl ProgressLogger for SilentProgressLogger {
    fn set_is_encode(&mut self, _is_encode: bool) {}
    fn log(&mut self, _num_input_bytes: usize, _num_output_bytes: usize) {}
    fn finish(&mut self, _num_input_bytes: usize, _num_output_bytes: usize) {}
}

pub struct SimpleProgressLogger {
    is_encode: bool,
    start_time: Instant,
    update_time: Instant,
    cur_num_input_bytes: usize,
    cur_num_output_bytes: usize,
}

impl SimpleProgressLogger {
    pub fn new() -> Self {
        SimpleProgressLogger {
            is_encode: true,
            start_time: Instant::now(),
            update_time: Instant::now(),
            cur_num_input_bytes: 0,
            cur_num_output_bytes: 0,
        }
    }
}

impl ProgressLogger for SimpleProgressLogger {
    fn set_is_encode(&mut self, is_encode: bool) {
        self.is_encode = is_encode;
    }

    fn log(&mut self, num_input_bytes: usize, num_output_bytes: usize) {
        let time = Instant::now();
        let batch_duration_micros = time.duration_since(self.update_time).as_micros();
        let ibs = num_input_bytes - self.cur_num_input_bytes;
        let obs = num_output_bytes - self.cur_num_output_bytes;

        if self.is_encode {
            let mbps = ibs as f64 / batch_duration_micros as f64;
            eprintln!("encode: {ibs} bytes => {obs} bytes, {mbps:.3} MB/s");
        } else {
            let mbps = obs as f64 / batch_duration_micros as f64;
            eprintln!("encode: {obs} bytes <= {ibs} bytes, {mbps:.3} MB/s");
        }
        self.cur_num_input_bytes = num_input_bytes;
        self.cur_num_output_bytes = num_output_bytes;
        self.update_time = time;
    }

    fn finish(&mut self, num_input_bytes: usize, num_output_bytes: usize) {
        self.cur_num_input_bytes = num_input_bytes;
        self.cur_num_output_bytes = num_output_bytes;
        self.update_time = Instant::now();

        let duration_micros = self.update_time.duration_since(self.start_time).as_micros();
        let ibs = self.cur_num_input_bytes;
        let obs = self.cur_num_output_bytes;
        let (ratio, mbps) = if self.is_encode {
            (
                obs as f64 * 100.0 / ibs as f64,
                ibs as f64 / duration_micros as f64,
            )
        } else {
            (
                ibs as f64 * 100.0 / obs as f64,
                obs as f64 / duration_micros as f64,
            )
        };

        eprintln!("statistics:");
        if self.is_encode {
            eprintln!("  size:  {ibs} bytes => {obs} bytes");
        } else {
            eprintln!("  size:  {obs} bytes <= {ibs} bytes");
        }
        eprintln!("  ratio: {ratio:.2}%");
        eprintln!("  speed: {:.3} MB/s", mbps);
        eprintln!("  time:  {:.3} sec", duration_micros as f64 * 1e-6);
    }
}
