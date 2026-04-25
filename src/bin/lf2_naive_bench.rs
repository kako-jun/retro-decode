//! Naive backward linear-scan LZSS bench (strict vs equal) against real LF2 corpus.

use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;

use retro_decode::formats::toheart::Lf2Image;

#[derive(Default)]
struct Stats {
    total: usize,
    binary_match: usize,
    payload_match: usize,
    total_payload_diff: u64,
    payload_diff_counted: u64,
}

impl Stats {
    fn record(&mut self, original: &[u8], reenc: &[u8]) {
        self.total += 1;
        if original == reenc {
            self.binary_match += 1;
        }
        if original.len() <= 0x16 {
            return;
        }
        let color_count = original[0x16] as usize;
        let payload_start = 0x18 + color_count * 3;
        if payload_start >= original.len() || payload_start >= reenc.len() {
            return;
        }
        let a = &original[payload_start..];
        let b = &reenc[payload_start..];
        if a == b {
            self.payload_match += 1;
        } else {
            let ml = a.len().min(b.len());
            let mut c = 0u64;
            for i in 0..ml {
                if a[i] != b[i] {
                    c += 1;
                }
            }
            c += (a.len().max(b.len()) - ml) as u64;
            self.total_payload_diff += c;
            self.payload_diff_counted += 1;
        }
    }

    fn print(&self, name: &str) {
        let avg = if self.payload_diff_counted > 0 {
            self.total_payload_diff as f64 / self.payload_diff_counted as f64
        } else {
            0.0
        };
        let pct = if self.total > 0 {
            self.payload_match as f64 * 100.0 / self.total as f64
        } else {
            0.0
        };
        println!("Variant: {}", name);
        println!("  total files : {}", self.total);
        println!("  binary match: {}", self.binary_match);
        println!(
            "  payload match: {} ({}/{} = {:.2}%)",
            self.payload_match, self.payload_match, self.total, pct
        );
        println!("  avg payload diff (over mismatches): {:.2} bytes", avg);
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        eprintln!("usage: {} <input_dir>", args[0]);
        return ExitCode::from(2);
    }
    let dir = PathBuf::from(&args[1]);
    if !dir.is_dir() {
        eprintln!("error: {} is not a directory", dir.display());
        return ExitCode::from(2);
    }

    let mut entries: Vec<PathBuf> = match fs::read_dir(&dir) {
        Ok(rd) => rd
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| {
                p.extension()
                    .and_then(|e| e.to_str())
                    .map(|s| s.eq_ignore_ascii_case("lf2"))
                    .unwrap_or(false)
            })
            .collect(),
        Err(e) => {
            eprintln!("error: read_dir failed: {}", e);
            return ExitCode::from(1);
        }
    };
    entries.sort();

    let mut strict = Stats::default();
    let mut equal = Stats::default();
    let mut errored = 0usize;

    for path in &entries {
        let name = path.file_name().and_then(|s| s.to_str()).unwrap_or("?");
        let original = match fs::read(path) {
            Ok(b) => b,
            Err(e) => {
                eprintln!("read fail {}: {}", name, e);
                errored += 1;
                continue;
            }
        };
        let lf2 = match Lf2Image::from_data(&original) {
            Ok(img) => img,
            Err(e) => {
                eprintln!("decode fail {}: {}", name, e);
                errored += 1;
                continue;
            }
        };
        match lf2.to_lf2_bytes_naive_strict() {
            Ok(b) => strict.record(&original, &b),
            Err(e) => {
                eprintln!("strict reenc fail {}: {}", name, e);
                errored += 1;
            }
        }
        match lf2.to_lf2_bytes_naive_equal() {
            Ok(b) => equal.record(&original, &b),
            Err(e) => {
                eprintln!("equal reenc fail {}: {}", name, e);
                errored += 1;
            }
        }
    }

    strict.print("naive_strict");
    println!();
    equal.print("naive_equal");
    if errored > 0 {
        eprintln!("errors: {}", errored);
    }

    ExitCode::SUCCESS
}
