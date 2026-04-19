fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").unwrap() == "macos" {
        compile_speech_recognizer();
    }

    tauri_build::build()
}

fn compile_speech_recognizer() {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let swift_dir = std::path::PathBuf::from(&manifest_dir).join("swift");
    let out_dir = std::env::var("OUT_DIR").unwrap();
    let lib_path = std::path::PathBuf::from(&out_dir).join("libSpeechRecognizer.a");

    let swift_source = swift_dir.join("SpeechRecognizer.swift");

    // Compile Swift source to static library
    let status = std::process::Command::new("swiftc")
        .args(&[
            "-emit-library",
            "-static",
            "-module-name", "SpeechRecognizer",
            "-target", "arm64-apple-macosx14.0",
            "-o", lib_path.to_str().unwrap(),
        ])
        .arg(swift_source.to_str().unwrap())
        .args(&["-framework", "Speech", "-framework", "AVFoundation"])
        .status()
        .expect("Failed to execute swiftc. Ensure Xcode command line tools are installed.");

    if !status.success() {
        panic!("Swift compilation failed. Check that Xcode and Swift are installed.");
    }

    // Link search path for the compiled library
    println!("cargo:rustc-link-search=native={}", out_dir);
    println!("cargo:rustc-link-lib=static=SpeechRecognizer");
    println!("cargo:rustc-link-lib=framework=Speech");
    println!("cargo:rustc-link-lib=framework=AVFoundation");

    // Rerun if Swift source changes
    println!("cargo:rerun-if-changed={}", swift_source.to_str().unwrap());
}