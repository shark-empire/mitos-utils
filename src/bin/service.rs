fn main() -> std::process::ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    mitos_utils::common::errors::run(
        "service",
        mitos_utils::applets::service::USAGE,
        args,
        mitos_utils::applets::service::run,
    )
}
