fn main() -> std::process::ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    mitos_utils::common::errors::run(
        "passwd",
        mitos_utils::applets::passwd::USAGE,
        args,
        mitos_utils::applets::passwd::run,
    )
}
