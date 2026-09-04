fn main() -> std::process::ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    mitos_utils::common::errors::run(
        "cat",
        mitos_utils::applets::cat::USAGE,
        args,
        mitos_utils::applets::cat::run,
    )
}
