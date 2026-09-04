fn main() -> std::process::ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    mitos_utils::common::errors::run(
        "sleep",
        mitos_utils::applets::sleep::USAGE,
        args,
        mitos_utils::applets::sleep::run,
    )
}
