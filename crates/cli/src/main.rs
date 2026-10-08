//! awscli-rust 진입점 (TESTCore `Program.cs`).

use awscli_rust_cli::{app, logging};

fn main() {
    logging::init();
    let args: Vec<String> = std::env::args_os()
        .skip(1)
        .map(|arg| arg.to_string_lossy().into_owned())
        .collect();
    std::process::exit(app::run(&args));
}
