fn main() -> std::process::ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    mitos_utils::common::errors::run(
        "env",
        mitos_utils::applets::env::USAGE,
        args,
        mitos_utils::applets::env::run,
    )
}
