fn main() -> std::process::ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    mitos_utils::common::errors::run(
        "dmesg",
        mitos_utils::applets::dmesg::USAGE,
        args,
        mitos_utils::applets::dmesg::run,
    )
}
