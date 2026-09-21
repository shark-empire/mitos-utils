fn main() -> std::process::ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    mitos_utils::common::errors::run(
        "false",
        mitos_utils::applets::false_::USAGE,
        args,
        mitos_utils::applets::false_::run,
    )
}
