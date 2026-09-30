//! soundcheck: play WAVs through EXACTLY the player's audio path (rodio,
//! amplify 0.5, convert_samples) so a human can A/B against afplay.
//!
//! Usage: cargo run -p app --bin soundcheck -- <file.wav> [more...]

use rodio::Source;

fn main() {
    let (_stream, handle) = rodio::OutputStream::try_default().expect("audio out");
    for path in std::env::args().skip(1) {
        println!(">> {path}");
        let f = std::fs::File::open(&path).expect("open wav");
        let dec = rodio::Decoder::new(std::io::BufReader::new(f)).expect("decode wav");
        let dur = dec
            .total_duration()
            .unwrap_or(std::time::Duration::from_secs(3));
        handle
            .play_raw(dec.amplify(0.5).convert_samples())
            .expect("play");
        std::thread::sleep(dur + std::time::Duration::from_millis(400));
    }
    println!("done");
}
