fn main() -> std::process::ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    mitos_utils::common::errors::run(
        "groupadd",
        mitos_utils::applets::groupadd::USAGE,
        args,
        mitos_utils::applets::groupadd::run,
    )
}
