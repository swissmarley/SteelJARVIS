pub mod engine;
pub mod greeting;

pub use engine::{AgentEngine, build_context, generate_greeting};
pub use greeting::try_greet;