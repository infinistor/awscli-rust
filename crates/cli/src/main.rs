//! awscli-rest 진입점 (TESTCore `Program.cs`).

mod app;
mod logging;
mod version;

fn main() {
    logging::init();
    let args: Vec<String> = std::env::args_os()
        .skip(1)
        .map(|arg| arg.to_string_lossy().into_owned())
        .collect();
    std::process::exit(app::run(&args));
}
