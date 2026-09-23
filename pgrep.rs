fn main() -> std::process::ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    mitos_utils::common::errors::run(
        "pgrep",
        mitos_utils::applets::pgrep::USAGE,
        args,
        mitos_utils::applets::pgrep::run,
    )
}
