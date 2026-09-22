//! `service` -- a thin client for mitos-services' control socket.
//!
//! Faithfully mirrors the real wire protocol (read directly from that
//! project's own `src/ipc.rs`) rather than guessing at a
//! start/stop/restart-per-unit interface that daemon doesn't actually
//! expose yet: connect, send one line (`COMMAND [ARGS]\n`), read the
//! full response until the connection closes, done -- no persistent
//! session, the same way `mitosctl` itself talks to this socket.
//! `RELOAD`/`ISOLATE` act on the whole supervisor (reload the config /
//! switch target), not on a single named unit -- there's currently no
//! per-unit start/stop/restart command to wrap, because the daemon
//! itself doesn't have one.

use crate::common::errors::{AppError, AppResult};
use std::io::{Read, Write};
use std::os::unix::net::UnixStream;
use std::time::Duration;

pub const USAGE: &str = "service {status|reload|ping|targets|apps|isolate TARGET|launch PATH [ARGS...]|logs [FILTER]} -- talk to mitos-services";

const SOCKET_PATH: &str = "/run/mitos-services/control.sock";

fn send_command(command: &str) -> AppResult<String> {
    let mut stream = UnixStream::connect(SOCKET_PATH).map_err(|e| {
        AppError::new(format!(
            "cannot reach mitos-services at {SOCKET_PATH}: {e} (is it running?)"
        ))
    })?;
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .map_err(|e| AppError::new(format!("cannot set socket timeout: {e}")))?;

    stream
        .write_all(format!("{command}\n").as_bytes())
        .map_err(|e| AppError::new(format!("cannot send command: {e}")))?;

    let mut response = String::new();
    stream
        .read_to_string(&mut response)
        .map_err(|e| AppError::new(format!("cannot read response: {e}")))?;
    Ok(response)
}

/// The wire protocol is one line in, so a literal newline inside an
/// argument would either get silently dropped by the daemon's
/// `read_line` (everything after it is never sent) or, depending on
/// buffering, confuse where one command ends -- rejected outright
/// instead of sending something that behaves ambiguously.
fn check_arg(arg: &str) -> AppResult<&str> {
    if arg.contains('\n') {
        Err(AppError::usage("arguments can't contain a newline"))
    } else {
        Ok(arg)
    }
}

pub fn run(args: Vec<String>) -> AppResult<()> {
    let (opts, forced) = crate::common::args::split_dashdash(args);
    let mut parts = opts;
    parts.extend(forced);

    let Some((subcommand, rest)) = parts.split_first() else {
        return Err(AppError::usage("missing subcommand"));
    };
    for arg in rest {
        check_arg(arg)?;
    }

    let command = match subcommand.as_str() {
        "status" => "STATUS".to_string(),
        "reload" => "RELOAD".to_string(),
        "ping" => "PING".to_string(),
        "targets" => "TARGETS".to_string(),
        "apps" => "APPS".to_string(),
        "isolate" => {
            let target = rest
                .first()
                .ok_or_else(|| AppError::usage("isolate requires a target name"))?;
            format!("ISOLATE {target}")
        }
        "launch" => {
            let path = rest
                .first()
                .ok_or_else(|| AppError::usage("launch requires a path"))?;
            let mut cmd = format!("LAUNCH {path}");
            for arg in &rest[1..] {
                cmd.push(' ');
                cmd.push_str(arg);
            }
            cmd
        }
        "logs" => format!("LOGS {}", rest.first().map(String::as_str).unwrap_or("")),
        other => return Err(AppError::usage(format!("unknown subcommand '{other}'"))),
    };

    print!("{}", send_command(&command)?);
    Ok(())
}
