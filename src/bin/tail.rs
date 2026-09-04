fn main() -> std::process::ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    mitos_utils::common::errors::run(
        "tail",
        mitos_utils::applets::tail::USAGE,
        args,
        mitos_utils::applets::tail::run,
    )
}
