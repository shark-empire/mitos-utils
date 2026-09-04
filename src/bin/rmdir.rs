fn main() -> std::process::ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    mitos_utils::common::errors::run(
        "rmdir",
        mitos_utils::applets::rmdir::USAGE,
        args,
        mitos_utils::applets::rmdir::run,
    )
}
