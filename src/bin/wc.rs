fn main() -> std::process::ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    mitos_utils::common::errors::run(
        "wc",
        mitos_utils::applets::wc::USAGE,
        args,
        mitos_utils::applets::wc::run,
    )
}
