fn main() -> std::process::ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    mitos_utils::common::errors::run("touch", mitos_utils::applets::touch::USAGE, args, mitos_utils::applets::touch::run)
}
