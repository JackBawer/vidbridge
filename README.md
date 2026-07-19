# vidbridge

A video processing library that bridges FFmpeg's C API to a safe Rust
interface. It decodes, encodes, and muxes video using FFmpeg under the hood,
supporting H.264 and H.265, and reads from local files or RTSP streams. The
C layer wraps FFmpeg; the Rust layer exposes a safe API on top of it via FFI,
with both a synchronous and an async (Tokio) interface.

## Requirements

- FFmpeg development libraries (`libavformat`, `libavcodec`, `libavutil`)
- CMake ≥ 3.16
- A C compiler (GCC or Clang)
- Rust (via [rustup](https://rustup.rs))
- `clang`/`libclang` (needed by `bindgen`)
- [`mediamtx`](https://github.com/bluenviron/mediamtx) (optional, only needed to test RTSP input locally)

## Setup

Clone the repo:
```sh
git clone https://github.com/JackBawer/vidbridge.git
cd vidbridge
```

Build the Rust crate (this also builds the C library via CMake automatically):
```sh
cd rust
cargo build
```

## Running

### File input (H.264 or H.265 output)

```sh
cargo run --bin transcode -- ../samples/sample03_hevc.mp4 output.mp4 h265
```

Arguments: `<input path or rtsp:// url> [output.mp4] [codec: h264|h265] [max_frames]`

### RTSP input

RTSP sources never signal end-of-stream on their own, so a frame limit
(5th argument) is required to stop processing. To test locally, run a
self-hosted RTSP server:

```sh
# Terminal 1: start a local RTSP server
mediamtx

# Terminal 2: push a sample file into it as a looping RTSP stream
ffmpeg -re -stream_loop -1 -i ../samples/sample01.mp4 -c copy -f rtsp rtsp://localhost:8554/live

# Terminal 3: run the transcode against it, capped at 300 frames
cargo run --bin transcode -- rtsp://localhost:8554/live output.mp4 h264 300
```

### C library standalone

```sh
cmake -S . -B build
cmake --build build
```
