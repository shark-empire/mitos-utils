fn main() -> std::process::ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    mitos_utils::common::errors::run(
        "sudo",
        mitos_utils::applets::sudo::USAGE,
        args,
        mitos_utils::applets::sudo::run,
    )
}
