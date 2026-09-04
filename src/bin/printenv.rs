fn main() -> std::process::ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    mitos_utils::common::errors::run(
        "printenv",
        mitos_utils::applets::printenv::USAGE,
        args,
        mitos_utils::applets::printenv::run,
    )
}
