fn main() -> std::process::ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    mitos_utils::common::errors::run("chmod", mitos_utils::applets::chmod::USAGE, args, mitos_utils::applets::chmod::run)
}
