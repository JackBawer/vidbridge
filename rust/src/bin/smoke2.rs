use vidbridge::{Decoder, Demuxer, Encoder, Frame, Muxer};

fn main() {
    let mut demuxer = Demuxer::open("../samples/sample04_hevc.mp4").unwrap();

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
        "libx265",
        demuxer.width(),
        demuxer.height(),
        demuxer.framerate(),
        2_000_000,
        true,
    )
    .unwrap();

    let mut muxer =
        Muxer::create("async_output_02.mp4", &encoder, demuxer.framerate()).unwrap();

    let mut frame = Frame::new().unwrap();

    let (mut decoded, mut encoded) = (0, 0);

    while let Some((packet, pts)) = demuxer.read_packet() {
        decoder.send_packet(Some(&packet), pts).unwrap();

        loop {
            if !decoder.receive_frame(&mut frame).unwrap() {
                break;
            }

            decoded += 1;

            encoder.send_frame(Some(&frame)).unwrap();

            while let Some((packet, pts, dts)) = encoder.receive_packet().unwrap() {
                encoded += 1;
                muxer
                    .write_packet(&packet, pts, dts, encoder.time_base())
                    .unwrap();
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

        while let Some((packet, pts, dts)) = encoder.receive_packet().unwrap() {
            encoded += 1;
            muxer
                .write_packet(&packet, pts, dts, encoder.time_base())
                .unwrap();
        }
    }

    encoder.send_frame(None).ok();

    while let Some((packet, pts, dts)) = encoder.receive_packet().unwrap() {
        encoded += 1;
        muxer
            .write_packet(&packet, pts, dts, encoder.time_base())
            .unwrap();
    }

    println!(
        "decoded {} frames, encoded {} frames -> async_output_02.mp4",
        decoded, encoded
    );
}
