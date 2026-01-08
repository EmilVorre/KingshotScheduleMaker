use std::collections::HashSet;
use std::env;
use std::fs;
use std::process::Command;
use std::path::Path;
use regex::Regex;
use ffmpeg_next as ffmpeg;
use image::{ImageBuffer, Rgb};

fn main() {
    ffmpeg::init().unwrap();

    let args: Vec<String> = env::args().collect();
    let frame_skip = if args.len() > 1 {
        args[1].parse::<usize>().unwrap_or(60)
    } else {
        60
    };

    let video_path = "Recording-2025-05-25-220805.mp4";
    let output_dir = "frames";
    let _ = fs::create_dir_all(output_dir);

    let mut ictx = ffmpeg::format::input(&video_path).expect("Failed to open video");
    let input = ictx.streams().best(ffmpeg::media::Type::Video).expect("No video stream found");
    let video_stream_index = input.index();
    let context_decoder = ffmpeg::codec::context::Context::from_parameters(input.parameters()).unwrap();
    let mut decoder = context_decoder.decoder().video().unwrap();

    let mut frame_index = 0;
    let mut decoded = ffmpeg::util::frame::video::Video::empty();
    let mut scaler = None;

    for (stream, packet) in ictx.packets() {
        if stream.index() != video_stream_index {
            continue;
        }

        decoder.send_packet(&packet).unwrap();

        while decoder.receive_frame(&mut decoded).is_ok() {
            if frame_index % frame_skip == 0 {
                if scaler.is_none() {
                    scaler = Some(
                        ffmpeg::software::scaling::context::Context::get(
                            decoder.format(),
                            decoder.width(),
                            decoder.height(),
                            ffmpeg::format::Pixel::RGB24,
                            decoder.width(),
                            decoder.height(),
                            ffmpeg::software::scaling::flag::Flags::BILINEAR,
                        ).unwrap(),
                    );
                }

                let mut rgb_frame = ffmpeg::util::frame::video::Video::empty();
                scaler.as_mut().unwrap().run(&decoded, &mut rgb_frame).unwrap();

                let buffer = rgb_frame.data(0);
                let width = rgb_frame.width();
                let height = rgb_frame.height();
                let img_buffer = ImageBuffer::<Rgb<u8>, _>::from_raw(width, height, buffer.to_vec()).unwrap();
                let file_path = format!("{}/frame_{:03}.png", output_dir, frame_index);
                img_buffer.save(&file_path).unwrap();
            }
            frame_index += 1;
        }
    }

    decoder.send_eof().unwrap();

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
