// tests/pipeline_test.rs
use vidbridge::{Demuxer, Decoder, Encoder, Muxer, Frame};

const SAMPLE: &str = "../samples/sample01.mp4";
const EXPECTED_FRAMES: i32 = 120;

#[test]
fn demuxer_opens_valid_file() {
    let demuxer = Demuxer::open(SAMPLE).expect("should open");
    assert_eq!(demuxer.width(), 1920);
    assert_eq!(demuxer.height(), 1080);
    assert_eq!(demuxer.codec_name(), "h264");
}

#[test]
fn demuxer_rejects_missing_file() {
    assert!(Demuxer::open("this_file_does_not_exist.mp4").is_err());
}

#[test]
fn decoder_rejects_bogus_codec_name() {
    assert!(Decoder::new("not_a_real_codec").is_err());
}

#[test]
fn full_decode_matches_expected_frame_count() {
    let count = decode_full(SAMPLE);
    assert_eq!(count, EXPECTED_FRAMES);
}

#[test]
fn repeat_decode_is_consistent() {
    let count1 = decode_full(SAMPLE);
    let count2 = decode_full(SAMPLE);
    assert_eq!(count1, count2);
    assert_eq!(count1, EXPECTED_FRAMES);
}

#[test]
fn frame_data_is_sane() {
    let mut demuxer = Demuxer::open(SAMPLE).expect("open failed");
    let mut decoder = Decoder::new(&demuxer.codec_name()).expect("decoder create failed");
    decoder.init_from_demuxer(&demuxer).expect("decoder init failed");

    let mut frame = Frame::new().expect("frame create failed");
    let mut got_frame = false;

    while !got_frame {
        let Some((packet, pts)) = demuxer.read_packet() else { break };
        decoder.send_packet(Some(&packet), pts).expect("send_packet failed");
        if let Ok(true) = decoder.receive_frame(&mut frame) {
            got_frame = true;
        }
    }

    assert!(got_frame, "expected at least one frame");
    let y_plane = frame.data(0);
    assert!(y_plane.is_some(), "Y plane should be non-null");
    let linesize = frame.linesize(0).expect("linesize should be valid");
    assert!(linesize >= demuxer.width(), "linesize should be at least frame width");
    assert!(frame.data(9).is_none(), "out-of-range plane should return None");
}

#[test]
fn full_transcode_and_mux_produces_playable_output() {
    let output_path = "test_transcode_output.mp4";

    let mut demuxer = Demuxer::open(SAMPLE).expect("open failed");
    let width = demuxer.width();
    let height = demuxer.height();
    let fps = demuxer.framerate();

    let mut decoder = Decoder::new(&demuxer.codec_name()).expect("decoder create failed");
    decoder.init_from_demuxer(&demuxer).expect("decoder init failed");

    let mut encoder = Encoder::new("libx264", width, height, fps, 2_000_000, true)
        .expect("encoder create failed");
    let mut muxer = Muxer::create(output_path, &encoder, fps).expect("muxer create failed");

    let mut frame = Frame::new().expect("frame create failed");
    let (mut decoded, mut encoded, mut muxed) = (0, 0, 0);

    let drain_encoder = |encoder: &mut Encoder, muxer: &mut Muxer, encoded: &mut i32, muxed: &mut i32| {
        loop {
            match encoder.receive_packet() {
                Ok(Some((data, pts, dts))) => {
                    *encoded += 1;
                    if muxer.write_packet(&data, pts, dts, encoder.time_base()).is_ok() {
                        *muxed += 1;
                    }
                }
                Ok(None) => break,
                Err(e) => panic!("encoder error: {e}"),
            }
        }
    };

    while let Some((packet, pts)) = demuxer.read_packet() {
        decoder.send_packet(Some(&packet), pts).expect("send_packet failed");
        loop {
            match decoder.receive_frame(&mut frame) {
                Ok(true) => {
                    decoded += 1;
                    encoder.send_frame(Some(&frame)).expect("send_frame failed");
                    drain_encoder(&mut encoder, &mut muxer, &mut encoded, &mut muxed);
                }
                Ok(false) => break,
                Err(e) => panic!("decode error: {e}"),
            }
        }
    }

    decoder.send_packet(None, 0).ok();
    loop {
        match decoder.receive_frame(&mut frame) {
            Ok(true) => {
                decoded += 1;
                encoder.send_frame(Some(&frame)).expect("send_frame failed");
                drain_encoder(&mut encoder, &mut muxer, &mut encoded, &mut muxed);
            }
            Ok(false) => break,
            Err(e) => panic!("decode error: {e}"),
        }
    }

    encoder.send_frame(None).ok();
    drain_encoder(&mut encoder, &mut muxer, &mut encoded, &mut muxed);

    assert_eq!(decoded, EXPECTED_FRAMES);
    assert_eq!(encoded, muxed, "every encoded packet should be muxed");
    assert!(encoded > 0);

    let _ = std::fs::remove_file(output_path); // cleanup
}

fn decode_full(path: &str) -> i32 {
    let mut demuxer = Demuxer::open(path).expect("open failed");
    let mut decoder = Decoder::new(&demuxer.codec_name()).expect("decoder create failed");
    decoder.init_from_demuxer(&demuxer).expect("decoder init failed");

    let mut frame = Frame::new().expect("frame create failed");
    let mut count = 0;

    while let Some((packet, pts)) = demuxer.read_packet() {
        decoder.send_packet(Some(&packet), pts).expect("send_packet failed");
        while let Ok(true) = decoder.receive_frame(&mut frame) {
            count += 1;
        }
    }

    decoder.send_packet(None, 0).ok();
    while let Ok(true) = decoder.receive_frame(&mut frame) {
        count += 1;
    }

    count
}

fn decode_full_with_ffprobe_check(path: &str) -> i32 {
    let mut demuxer = Demuxer::open(path).expect("open failed");
    let mut decoder = Decoder::new(&demuxer.codec_name()).expect("decoder create failed");
    decoder.init_from_demuxer(&demuxer).expect("decoder init failed");
    let mut frame = Frame::new().expect("frame create failed");
    let mut count = 0;
    while let Some((packet, pts)) = demuxer.read_packet() {
        decoder.send_packet(Some(&packet), pts).expect("send_packet failed");
        while let Ok(true) = decoder.receive_frame(&mut frame) {
            count += 1;
        }
    }
    decoder.send_packet(None, 0).ok();
    while let Ok(true) = decoder.receive_frame(&mut frame) {
        count += 1;
    }
    count
}

#[test]
fn decode_mkv_container() {
    let count = decode_full_with_ffprobe_check("../samples/sample02.mkv");
    assert!(count > 0, "expected at least one frame from MKV source");
}

#[test]
fn decode_raw_hevc_elementary_stream() {
    let mut demuxer = Demuxer::open("../samples/sample03.hevc").expect("open failed");
    assert_eq!(demuxer.codec_name(), "hevc");

    let mut decoder = Decoder::new("hevc").expect("decoder create failed");
    decoder.init_from_demuxer(&demuxer).expect("decoder init failed");

    let mut frame = Frame::new().expect("frame create failed");
    let mut count = 0;

    while let Some((packet, pts)) = demuxer.read_packet() {
        decoder.send_packet(Some(&packet), pts).expect("send_packet failed");
        while let Ok(true) = decoder.receive_frame(&mut frame) {
            count += 1;
        }
    }
    decoder.send_packet(None, 0).ok();
    while let Ok(true) = decoder.receive_frame(&mut frame) {
        count += 1;
    }

    assert!(count > 0, "expected at least one frame from raw HEVC stream");
}

#[test]
fn encode_hevc_output() {
    let mut demuxer = Demuxer::open("../samples/sample01.mp4").expect("open failed");
    let width = demuxer.width();
    let height = demuxer.height();
    let fps = demuxer.framerate();

    let mut decoder = Decoder::new(&demuxer.codec_name()).expect("decoder create failed");
    decoder.init_from_demuxer(&demuxer).expect("decoder init failed");

    let mut encoder = Encoder::new("libx265", width, height, fps, 1_500_000, true)
        .expect("HEVC encoder create failed");

    // Muxer is created lazily, after the first encoded packet is available —
    // x265 does not guarantee codec_ctx->extradata (VPS/SPS/PPS) is fully
    // populated immediately after avcodec_open2, unlike x264. Creating the
    // muxer too early copies incomplete extradata into the container header,
    // producing files that fail to parse the first NAL unit on playback.
    let mut muxer: Option<Muxer> = None;

    let mut frame = Frame::new().expect("frame create failed");
    let mut decoded = 0;
    let mut encoded = 0;

    let mut handle_packets = |encoder: &mut Encoder, muxer: &mut Option<Muxer>, encoded: &mut i32| {
        while let Ok(Some((data, pts, dts))) = encoder.receive_packet() {
            if muxer.is_none() {
                *muxer = Some(
                    Muxer::create("test_hevc_output.mp4", encoder, fps)
                        .expect("muxer create failed"),
                );
            }
            *encoded += 1;
            let _ = muxer.as_mut().unwrap().write_packet(&data, pts, dts, encoder.time_base());
        }
    };

    while let Some((packet, pts)) = demuxer.read_packet() {
        decoder.send_packet(Some(&packet), pts).expect("send_packet failed");
        while let Ok(true) = decoder.receive_frame(&mut frame) {
            decoded += 1;
            if encoder.send_frame(Some(&frame)).is_ok() {
                handle_packets(&mut encoder, &mut muxer, &mut encoded);
            }
        }
    }

    decoder.send_packet(None, 0).ok();
    while let Ok(true) = decoder.receive_frame(&mut frame) {
        decoded += 1;
        if encoder.send_frame(Some(&frame)).is_ok() {
            handle_packets(&mut encoder, &mut muxer, &mut encoded);
        }
    }

    encoder.send_frame(None).ok();
    handle_packets(&mut encoder, &mut muxer, &mut encoded);

    assert_eq!(decoded, 120, "decoder should produce all 120 source frames");
    assert_eq!(encoded, 120, "encoder should produce all 120 frames after full flush");

    // let _ = std::fs::remove_file("test_hevc_output.mp4");
}
