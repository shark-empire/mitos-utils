fn main() -> std::process::ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    mitos_utils::common::errors::run("cut", mitos_utils::applets::cut::USAGE, args, mitos_utils::applets::cut::run)
}
