fn main() -> std::process::ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    mitos_utils::common::errors::run("chgrp", mitos_utils::applets::chgrp::USAGE, args, mitos_utils::applets::chgrp::run)
}
