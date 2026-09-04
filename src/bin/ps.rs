fn main() -> std::process::ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    mitos_utils::common::errors::run(
        "ps",
        mitos_utils::applets::ps::USAGE,
        args,
        mitos_utils::applets::ps::run,
    )
}
