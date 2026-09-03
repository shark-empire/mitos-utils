fn main() -> std::process::ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    mitos_utils::common::errors::run("pwd", mitos_utils::applets::pwd::USAGE, args, mitos_utils::applets::pwd::run)
}
