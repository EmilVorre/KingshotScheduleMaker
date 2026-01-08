use std::collections::HashSet;
use std::env;
use std::path::Path;
use std::process::Command;
use std::fs;
use regex::Regex;

fn main() {
    // Read frame skip interval (in frames) from command line args or default to 60
    let args: Vec<String> = env::args().collect();
    let frame_skip = if args.len() > 1 {
        args[1].parse::<u32>().unwrap_or(60)
    } else {
        60
    };

    let video_path = "Recording-2025-05-25-220805.mp4";
    let output_dir = "frames";

    // Extract frames from video using ffmpeg
    println!("Extracting every {} frame(s)...", frame_skip);
    let _ = fs::create_dir_all(output_dir);
    Command::new("ffmpeg")
        .args(["-i", video_path, "-vf", &format!("select='not(mod(n\\,{}))',setpts=N/FRAME_RATE/TB", frame_skip), "-vsync", "vfr", "frames/frame_%03d.png"])
        .output()
        .expect("Failed to extract frames with ffmpeg");

    // Prepare regex
    let re = Regex::new(r"\[([A-Za-z0-9]+)\]\s+([^\d\n]+)\s+([\d,]+)").unwrap();
    let mut seen = HashSet::new();

    println!("Scanning frames...");
    for entry in fs::read_dir(output_dir).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().map(|s| s == "png").unwrap_or(false) {
            let output = Command::new("tesseract")
                .arg(&path)
                .arg("stdout")
                .output()
                .expect("Failed to run tesseract");
            let text = String::from_utf8_lossy(&output.stdout);

            for cap in re.captures_iter(&text) {
                let tag = &cap[1];
                let name = cap[2].trim();
                let power = &cap[3];
                let full = format!("[{tag}] {name} - {power}");
                if seen.insert(full.clone()) {
                    println!("{}", full);
                }
            }
        }
    }
    println!("Done.");
}
