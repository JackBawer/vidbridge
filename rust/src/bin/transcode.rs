use vidbridge::{Decoder, Demuxer, Encoder, Frame, Muxer};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!(
            "usage: {} <input path or rtsp://url> [output.mp4] [codec: h264|h265] [max_frames]",
            args[0]
        );
        eprintln!("  max_frames: stop after N frames (required for RTSP, which never reaches EOF on its own)");
        std::process::exit(1);
    }

    let input = &args[1];
    let output = args.get(2).map(String::as_str).unwrap_or("output.mp4");
    let codec_arg = args.get(3).map(String::as_str).unwrap_or("h265");
    let encoder_name = match codec_arg {
        "h264" => "libx264",
        "h265" => "libx265",
        other => {
            eprintln!("unknown codec '{other}', expected h264 or h265");
            std::process::exit(1);
        }
    };
    let max_frames: Option<usize> = args.get(4).and_then(|s| s.parse().ok());

    let mut demuxer = Demuxer::open(input).unwrap();
    println!(
        "opened {}x{} at {}/{} codec={}",
        demuxer.width(),
        demuxer.height(),
        demuxer.framerate().num,
        demuxer.framerate().den,
        demuxer.codec_name()
    );

    let codec = demuxer.codec_name();
    let mut decoder = Decoder::new(&codec).unwrap();
    decoder.init_from_demuxer(&demuxer).unwrap();

    let mut encoder = Encoder::new(
        encoder_name,
        demuxer.width(),
        demuxer.height(),
        demuxer.framerate(),
        2_000_000,
        true,
    )
    .unwrap();

    let mut muxer = Muxer::create(output, &encoder, demuxer.framerate()).unwrap();
    let mut frame = Frame::new().unwrap();
    let (mut decoded, mut encoded) = (0, 0);

    'capture: while let Some((packet, pts)) = demuxer.read_packet() {
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

            if let Some(limit) = max_frames {
                if decoded >= limit {
                    break 'capture;
                }
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

    println!("decoded {decoded} frames, encoded {encoded} frames -> {output}");
}
