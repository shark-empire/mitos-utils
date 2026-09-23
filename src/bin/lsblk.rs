fn main() -> std::process::ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    mitos_utils::common::errors::run(
        "lsblk",
        mitos_utils::applets::lsblk::USAGE,
        args,
        mitos_utils::applets::lsblk::run,
    )
}
