//! A minimal `ffplay`-equivalent built on vidbridge's own demux/decode
//! pipeline instead of shelling out to FFmpeg's player. Opens a file or
//! RTSP URL via the existing `Demuxer`/`Decoder`, and renders each decoded
//! YUV420P frame directly to an SDL3 window in real time -- no encoding,
//! no muxing, just decode-and-display.
//!
//! usage: play <input: file path or rtsp:// url>
//!
//! Requires SDL3 to be installed on the system:
//!   Arch: sudo pacman -S sdl3
//!
//! Uses the zero-copy read_packet_ref path (see demuxer.rs) since each
//! packet here is used exactly once, immediately, to feed the decoder --
//! the same pattern as transcode.rs.

use sdl3::event::Event;
use sdl3::pixels::PixelFormat;
use sdl3::render::TextureAccess;
use std::time::{Duration, Instant};
use vidbridge::{Decoder, Demuxer, Frame};

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 {
        eprintln!("usage: {} <input: file path or rtsp:// url>", args[0]);
        eprintln!("  for RTSP with credentials, embed them in the URL as usual:");
        eprintln!("  rtsp://user:pass@host:554/path");
        std::process::exit(1);
    }

    let input = &args[1];

    let mut demuxer = Demuxer::open(input).expect("failed to open source");

    let width = demuxer.width();
    let height = demuxer.height();
    let codec_name = demuxer.codec_name();
    let framerate = demuxer.framerate();

    // Fall back to ~30 FPS if the source doesn't report a usable
    // framerate. This also avoids division by zero.
    let frame_duration = if framerate.num > 0 && framerate.den > 0 {
        Duration::from_secs_f64(
            framerate.den as f64 / framerate.num as f64,
        )
    } else {
        Duration::from_millis(33)
    };

    println!(
        "opened {width}x{height} codec={codec_name} at {}/{}",
        framerate.num, framerate.den
    );

    let mut decoder =
        Decoder::new(&codec_name).expect("failed to create decoder");

    decoder
        .init_from_demuxer(&demuxer)
        .expect("failed to init decoder from demuxer");

    // ---------------------------------------------------------------------
    // SDL3 initialization
    // ---------------------------------------------------------------------

    let sdl_context = sdl3::init().expect("SDL3 init failed");

    let video_subsystem = sdl_context
        .video()
        .expect("SDL3 video subsystem init failed");

    let window = video_subsystem
        .window("vidbridge play", width as u32, height as u32)
        .position_centered()
        .resizable()
        .build()
        .expect("failed to create window");

    let mut canvas = window.into_canvas();

    let texture_creator = canvas.texture_creator();

    // IYUV is planar YUV420:
    //
    //   Y
    //   U
    //   V
    //
    // This matches the three planes returned by Frame::data(0/1/2).
    //
    // SDL3's Rust bindings provide create_texture_streaming() as a
    // shorthand for a streaming texture. YUV planar formats are supported
    // by the renderer.
    let mut texture = texture_creator
        .create_texture_streaming(
            PixelFormat::IYUV,
            width as u32,
            height as u32,
        )
        .expect("failed to create YUV texture");

    let mut event_pump = sdl_context
        .event_pump()
        .expect("failed to get SDL3 event pump");

    let mut frame = Frame::new().expect("frame alloc failed");

    // ---------------------------------------------------------------------
    // Playback
    // ---------------------------------------------------------------------

    'playback: loop {
        // SDL3 event handling.
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. } => {
                    break 'playback;
                }
                _ => {}
            }
        }

        let Some((packet, pts)) = demuxer.read_packet_ref() else {
            println!("end of stream");
            break;
        };

        // packet borrows demuxer and is consumed immediately.
        decoder
            .send_packet(Some(packet), pts)
            .expect("failed to send packet to decoder");

        while decoder
            .receive_frame(&mut frame)
            .expect("failed to receive decoded frame")
        {
            let render_start = Instant::now();

            let y_data = frame
                .data(0)
                .expect("missing Y plane");

            let u_data = frame
                .data(1)
                .expect("missing U plane");

            let v_data = frame
                .data(2)
                .expect("missing V plane");

            let y_stride =
                frame.linesize(0).expect("missing Y linesize") as usize;

            let u_stride =
                frame.linesize(1).expect("missing U linesize") as usize;

            let v_stride =
                frame.linesize(2).expect("missing V linesize") as usize;

            // Y is full height.
            let y_plane = unsafe {
                std::slice::from_raw_parts(
                    y_data,
                    y_stride * height as usize,
                )
            };

            // U and V are half-height for YUV420P.
            let chroma_height = (height as usize).div_ceil(2);

            let u_plane = unsafe {
                std::slice::from_raw_parts(
                    u_data,
                    u_stride * chroma_height,
                )
            };

            let v_plane = unsafe {
                std::slice::from_raw_parts(
                    v_data,
                    v_stride * chroma_height,
                )
            };

            // Upload the three decoder planes directly into the SDL3
            // streaming YUV texture.
            texture
                .update_yuv(
                    None,
                    y_plane,
                    y_stride,
                    u_plane,
                    u_stride,
                    v_plane,
                    v_stride,
                )
                .expect("texture update failed");

            canvas.clear();

            canvas
                .copy(&texture, None, None)
                .expect("canvas copy failed");

            canvas.present();

            // Pace playback according to the source framerate.
            //
            // Your camera reports 20/1, so this will target approximately
            // 50 ms per decoded frame.
            let elapsed = render_start.elapsed();

            if elapsed < frame_duration {
                std::thread::sleep(frame_duration - elapsed);
            }
        }
    }
}

