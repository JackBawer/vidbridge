// src/bin/transcode.rs
use vidbridge::{Demuxer, Decoder, Encoder, Muxer, Frame};
use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        eprintln!("usage: {} <input> <output.mp4>", args[0]);
        std::process::exit(1);
    }
    let input_path = &args[1];
    let output_path = &args[2];

    let mut demuxer = Demuxer::open(input_path).expect("failed to open input");
    let width = demuxer.width();
    let height = demuxer.height();
    let fps = demuxer.framerate();
    println!("input: {width}x{height} codec={}", demuxer.codec_name());

    let mut decoder = Decoder::new(&demuxer.codec_name()).expect("decoder create failed");
    decoder.init_from_demuxer(&demuxer).expect("decoder init failed");

    let mut encoder = Encoder::new("libx265", width, height, fps, 2_000_000, true)
        .expect("encoder create failed");
    let mut muxer = Muxer::create(output_path, &encoder, fps).expect("muxer create failed");

    let mut frame = Frame::new().expect("frame create failed");

    let drain = |encoder: &mut Encoder, muxer: &mut Muxer, encoded: &mut i32| {
        while let Ok(Some((data, pts, dts))) = encoder.receive_packet() {
            *encoded += 1;
            let _ = muxer.write_packet(&data, pts, dts, encoder.time_base());
        }
    };

    let mut decoded = 0;
    let mut encoded = 0;
    const MAX_FRAMES: i32 = 300; // stop after ~300 frames — a live/RTSP source never ends on its own

    while let Some((packet, pts)) = demuxer.read_packet() {
        if decoded >= MAX_FRAMES {
            break;
        }
        decoder.send_packet(Some(&packet), pts).expect("send_packet failed");
        while let Ok(true) = decoder.receive_frame(&mut frame) {
            decoded += 1;
            if encoder.send_frame(Some(&frame)).is_ok() {
                drain(&mut encoder, &mut muxer, &mut encoded);
            }
        }
    }

    decoder.send_packet(None, 0).ok();
    while let Ok(true) = decoder.receive_frame(&mut frame) {
        decoded += 1;
        if encoder.send_frame(Some(&frame)).is_ok() {
            drain(&mut encoder, &mut muxer, &mut encoded);
        }
    }

    encoder.send_frame(None).ok();
    drain(&mut encoder, &mut muxer, &mut encoded);

    println!("decoded {decoded} frames, encoded {encoded} frames -> {output_path}");
}
