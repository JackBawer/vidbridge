# Project description

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
