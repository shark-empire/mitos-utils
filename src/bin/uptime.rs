fn main() -> std::process::ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    mitos_utils::common::errors::run(
        "uptime",
        mitos_utils::applets::uptime::USAGE,
        args,
        mitos_utils::applets::uptime::run,
    )
}
