fn main() -> std::process::ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    mitos_utils::common::errors::run(
        "mv",
        mitos_utils::applets::mv::USAGE,
        args,
        mitos_utils::applets::mv::run,
    )
}
