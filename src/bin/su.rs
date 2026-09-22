fn main() -> std::process::ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    mitos_utils::common::errors::run(
        "su",
        mitos_utils::applets::su::USAGE,
        args,
        mitos_utils::applets::su::run,
    )
}
