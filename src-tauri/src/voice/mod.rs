pub mod speech;
pub mod clap_detector;
pub mod activation;
pub mod audio_capture;
pub mod stt;

pub use speech::SpeechManager;
pub use clap_detector::ClapDetector;
pub use activation::ActivationManager;
pub use audio_capture::AudioCapture;
pub use stt::SpeechRecognizer;