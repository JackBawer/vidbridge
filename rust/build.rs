use std::env;
use std::path::PathBuf;

fn main() {
    let dst = cmake::Config::new("..")
        .define("BUILD_TESTS", "OFF")
        .build();

    println!("cargo:rustc-link-search=native={}/build", dst.display());
    println!("cargo:rustc-link-lib=vidbridge");

    println!("cargo:rustc-link-lib=avcodec");
    println!("cargo:rustc-link-lib=avformat");
    println!("cargo:rustc-link-lib=avutil");

    let bindings = bindgen::Builder::default()
        .header("../include/video_wrapper.h")
        .clang_arg("-I../include")
        .allowlist_function("frame_.*")
        .allowlist_function("demuxer_.*")
        .allowlist_function("decoder_.*")
        .allowlist_function("encoder_.*")
        .allowlist_function("muxer_.*")
        .allowlist_type("AVRational")
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        .generate()
        .expect("Unable to generate bindings");

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");
        
}
