fn main() -> std::process::ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    mitos_utils::common::errors::run(
        "free",
        mitos_utils::applets::free::USAGE,
        args,
        mitos_utils::applets::free::run,
    )
}
