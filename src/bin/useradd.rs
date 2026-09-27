fn main() -> std::process::ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    mitos_utils::common::errors::run(
        "useradd",
        mitos_utils::applets::useradd::USAGE,
        args,
        mitos_utils::applets::useradd::run,
    )
}
