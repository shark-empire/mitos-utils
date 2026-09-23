fn main() -> std::process::ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    mitos_utils::common::errors::run(
        "ping",
        mitos_utils::applets::ping::USAGE,
        args,
        mitos_utils::applets::ping::run,
    )
}
