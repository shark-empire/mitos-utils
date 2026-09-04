fn main() -> std::process::ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    mitos_utils::common::errors::run(
        "ln",
        mitos_utils::applets::ln::USAGE,
        args,
        mitos_utils::applets::ln::run,
    )
}
