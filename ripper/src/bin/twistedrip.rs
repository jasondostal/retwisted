//! twistedrip CLI -- the Rust twin of tools/rip.py:
//!
//!     twistedrip <input> --out <dir> [--only <slug>]
//!
//! <input> is a .sit / .sit.hqx / .iso / MacBinary file, or a folder of
//! extracted module files. Output lines match rip.py's.

use std::path::PathBuf;
use std::process::ExitCode;
use std::time::Instant;

const USAGE: &str = "usage: twistedrip <input> --out <dir> [--only <slug>] [--list-forks]";

fn main() -> ExitCode {
    let mut input: Option<PathBuf> = None;
    let mut out: Option<PathBuf> = None;
    let mut only: Option<String> = None;
    let mut list_forks = false;
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        match a.as_str() {
            "--out" => out = args.next().map(PathBuf::from),
            "--only" => only = args.next(),
            "--list-forks" => list_forks = true,
            "-h" | "--help" => {
                println!("{USAGE}");
                return ExitCode::SUCCESS;
            }
            _ if input.is_none() && !a.starts_with("--") => input = Some(PathBuf::from(a)),
            _ => {
                eprintln!("{USAGE}");
                return ExitCode::from(2);
            }
        }
    }
    let Some(input) = input else {
        eprintln!("{USAGE}");
        return ExitCode::from(2);
    };

    if list_forks {
        // Debug aid: the ingest layer's result as name / length / FNV-1a.
        match twistedrip::ingest::ingest(&input, &mut |_| {}) {
            Ok(forks) => {
                for (name, fork) in forks.iter() {
                    let h = fork.iter().fold(0xcbf29ce484222325u64, |x, &b| {
                        (x ^ b as u64).wrapping_mul(0x100000001b3)
                    });
                    println!("{name}\t{}\t{h:016x}", fork.len());
                }
                return ExitCode::SUCCESS;
            }
            Err(e) => {
                eprintln!("error: {e}");
                return ExitCode::FAILURE;
            }
        }
    }

    let Some(out) = out else {
        eprintln!("{USAGE}");
        return ExitCode::from(2);
    };
    let t0 = Instant::now();
    let opts = twistedrip::Options { only: only.clone() };
    let report = match twistedrip::rip(&input, &out, &opts, &mut |_| {}) {
        Ok(r) => r,
        Err(twistedrip::Error::NoSuchModule(o)) => {
            eprintln!("error: --only {o:?} matched no module");
            return ExitCode::FAILURE;
        }
        Err(e) => {
            eprintln!("error: {e}");
            return ExitCode::FAILURE;
        }
    };
    for m in &report.modules {
        match &m.dir {
            None => println!(
                "{}: SKIP -- no \"{}\" resource fork found in {}",
                m.slug,
                m.mac_name,
                input.display()
            ),
            Some(dir) => {
                let music = if m.songs > 0 {
                    format!(", {} song{}", m.songs, if m.songs != 1 { "s" } else { "" })
                } else {
                    String::new()
                };
                println!(
                    "{}: {} series, {} compounds, {} sounds{music} -> {}",
                    m.slug,
                    m.series,
                    m.compounds,
                    m.sounds,
                    dir.display()
                );
            }
        }
    }
    match &report.faceplate {
        Some(p) => println!(
            "{}: faceplate -> {}",
            twistedrip::pack::SHARED_DIR,
            p.display()
        ),
        None => println!(
            "{}: WARN -- no faceplate PICT found in {}",
            twistedrip::pack::SHARED_DIR,
            input.display()
        ),
    }
    println!("done in {:.1}s", t0.elapsed().as_secs_f64());
    ExitCode::SUCCESS
}
