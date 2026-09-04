fn main() -> std::process::ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    mitos_utils::common::errors::run(
        "echo",
        mitos_utils::applets::echo::USAGE,
        args,
        mitos_utils::applets::echo::run,
    )
}
