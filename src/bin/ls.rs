fn main() -> std::process::ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    mitos_utils::common::errors::run(
        "ls",
        mitos_utils::applets::ls::USAGE,
        args,
        mitos_utils::applets::ls::run,
    )
}
