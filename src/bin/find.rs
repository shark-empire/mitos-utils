fn main() -> std::process::ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    mitos_utils::common::errors::run(
        "find",
        mitos_utils::applets::find::USAGE,
        args,
        mitos_utils::applets::find::run,
    )
}
