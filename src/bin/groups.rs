fn main() -> std::process::ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    mitos_utils::common::errors::run(
        "groups",
        mitos_utils::applets::groups::USAGE,
        args,
        mitos_utils::applets::groups::run,
    )
}
