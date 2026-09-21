fn main() -> std::process::ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    mitos_utils::common::errors::run(
        "date",
        mitos_utils::applets::date::USAGE,
        args,
        mitos_utils::applets::date::run,
    )
}
