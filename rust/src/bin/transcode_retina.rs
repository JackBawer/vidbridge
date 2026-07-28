//! Same job as `transcode.rs`, but sources RTSP via the pure-Rust `retina`
//! client instead of FFmpeg's built-in RTSP demuxer. Existing `transcode.rs`
//! is unchanged and still uses FFmpeg for everything, including RTSP.
//!
//! usage: transcode_retina <rtsp://url> [output.mp4] [codec: h264|h265] [max_frames] [username] [password]

use vidbridge::{Decoder, Encoder, Frame, Muxer, RetinaRtspSource};

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!(
            "usage: {} <rtsp://url> [output.mp4] [codec: h264|h265] [max_frames] [username] [password]",
            args[0]
        );
        eprintln!("  max_frames: stop after N frames (required — RTSP never reaches EOF on its own)");
        std::process::exit(1);
    }

    let input = &args[1];
    let output = args.get(2).map(String::as_str).unwrap_or("output_retina.mp4");
    let codec_arg = args.get(3).map(String::as_str).unwrap_or("h265");
    let encoder_name = match codec_arg {
        "h264" => "libx264",
        "h265" => "libx265",
        other => {
            eprintln!("unknown codec '{other}', expected h264 or h265");
            std::process::exit(1);
        }
    };
    let max_frames: usize = args
        .get(4)
        .and_then(|s| s.parse().ok())
        .unwrap_or(300);
    let username = args.get(5).map(String::as_str);
    let password = args.get(6).map(String::as_str);

    let mut source = RetinaRtspSource::open(input, username, password)
        .await
        .expect("failed to open RTSP stream via retina");

    println!(
        "opened {}x{} codec={} via retina",
        source.width(),
        source.height(),
        source.codec_name()
    );

    let mut decoder = Decoder::new(source.codec_name()).unwrap();
    decoder
        .init_from_extradata(source.extradata())
        .expect("failed to init decoder from retina extradata");

    let mut encoder = Encoder::new(
        encoder_name,
        source.width(),
        source.height(),
        vidbridge::AVRational { num: 30, den: 1 }, // RTSP framerate isn't in SDP; adjust if your camera exposes one
        2_000_000,
        true,
    )
    .unwrap();

    let mut muxer = Muxer::create(output, &encoder, vidbridge::AVRational { num: 30, den: 1 }).unwrap();
    let mut frame = Frame::new().unwrap();
    let (mut decoded, mut encoded) = (0, 0);

    'capture: while let Some((packet, pts)) = source.read_packet().await {
        decoder.send_packet(Some(&packet), pts).unwrap();

        loop {
            if !decoder.receive_frame(&mut frame).unwrap() {
                break;
            }
            decoded += 1;
            encoder.send_frame(Some(&frame)).unwrap();
            while let Some((pkt, pts, dts)) = encoder.receive_packet().unwrap() {
                encoded += 1;
                muxer.write_packet(&pkt, pts, dts, encoder.time_base()).unwrap();
            }

            if decoded >= max_frames {
                break 'capture;
            }
        }
    }

    decoder.send_packet(None, 0).ok();
    loop {
        if !decoder.receive_frame(&mut frame).unwrap() {
            break;
        }
        decoded += 1;
        encoder.send_frame(Some(&frame)).unwrap();
        while let Some((pkt, pts, dts)) = encoder.receive_packet().unwrap() {
            encoded += 1;
            muxer.write_packet(&pkt, pts, dts, encoder.time_base()).unwrap();
        }
    }

    encoder.send_frame(None).ok();
    while let Some((pkt, pts, dts)) = encoder.receive_packet().unwrap() {
        encoded += 1;
        muxer.write_packet(&pkt, pts, dts, encoder.time_base()).unwrap();
    }

    println!("decoded {decoded} frames, encoded {encoded} frames -> {output} (via retina)");
}
