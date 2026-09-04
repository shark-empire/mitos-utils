fn main() -> std::process::ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    mitos_utils::common::errors::run(
        "tee",
        mitos_utils::applets::tee::USAGE,
        args,
        mitos_utils::applets::tee::run,
    )
}
