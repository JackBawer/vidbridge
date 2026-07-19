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

Run a sample transcode using one of the example binaries:
```sh
cargo run --bin smoke2
```

To build and run the standalone C library:
```sh
cmake -S . -B build
cmake --build build
```
