✅  Learn build system commands and cmake and make files and all that
✅  Decode video and then decode it with the help of ffmpeg
✅  RTSP simulation to work from and test the library at the end
❌  Test for synchronous then asynchronous
✅  Use derive(repr C) (used binden crate instead)
❌  Rust Async 
✅  Retina ingest (Rust crate)
❌  DMA (Direct memory access)
❌  Diagram to describe async workflow/flowchart
❌  Basically desktop app to record in a directory with many files (with the
    ability to customise length and camera login and all that, steal from the
    existing version)
❌  Comment the code myself (remove all AI comments) rust doc style
❌  Video system design engineering + design patterns
      

# Project idea: 

implement ffmpeg encoding and decoding to support the different codecs
(h264/h265) + ffi with rust (tokio crate) 

So the wrapper should be ffmpeg and we work with Rust async such that the crate
will handle everything but the C code will be unsafe to Rust so we'll need to 
handle that.

# Project description:

The project aims to build a video processing library that integrates FFmpeg
with a C wrapper to provide encoding and decoding support for H.264 (AVC) and
H.265 (HEVC) video codecs. The C layer will encapsulate the FFmpeg API and
expose a stable C-compatible interface (`extern "C"`), hiding FFmpeg's
complexity and C implementation details. This interface will then be accessed
from Rust through Foreign Function Interface (FFI) bindings. On the Rust side,
a safe and ergonomic wrapper will be implemented around the unsafe FFI calls,
managing resource lifetimes and preventing memory safety issues. The library
will be designed to work within Rust's asynchronous ecosystem, using Tokio to
perform video processing without blocking asynchronous tasks, likely by
executing the synchronous FFmpeg operations on dedicated blocking threads. The
final result will be a Rust-friendly API capable of opening video files,
decoding frames, encoding video in H.264 and H.265 formats, and safely managing
communication between Rust and the underlying C/FFmpeg implementation.

# Baby steps:

1. Open a video and print its metadata.
2. Decode the first frame.
3. Decode every frame.
4. Encode a simple sequence of frames into H.264.
5. Repeat with H.265.
6. Wrap those operations behind a small C API.
7. Call the C API from Rust.
8. Add a safe Rust wrapper.
9. Integrate with Tokio.

# Commands to test retina {
    * terminal 1
        $ mediamtx


    * terminal 2
        $ ffmpeg -re -stream_loop -1 -i ../samples/sample01.mp4 -c copy -f rtsp rtsp://localhost:8554/live

    * terminal 3
        $ cargo run --bin transcode_retina -- rtsp://localhost:8554/live out_retina.mp4 h264 300
}


