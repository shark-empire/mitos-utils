//! `ping` -- send ICMP echo requests to a host.
//!
//! IPv4 only for now (ICMPv6 is a different protocol number with a
//! different checksum rule -- a real gap, not a silent one). Tries
//! an unprivileged Linux "ping socket" first (`SOCK_DGRAM` +
//! `IPPROTO_ICMP`, gated by the `net.ipv4.ping_group_range` sysctl)
//! and only falls back to a raw socket (needs root or `CAP_NET_RAW`)
//! if that's refused -- matching how modern `ping` implementations
//! already behave, and meaning this often needs no special
//! privilege at all on a system with a permissive `ping_group_range`.
//!
//! Hostname resolution goes through `std::net::ToSocketAddrs`
//! (backed by the system's own resolver) rather than a hand-rolled
//! DNS client or `getaddrinfo` FFI -- one function call instead of
//! reimplementing a whole protocol, matching this crate's `crypt()`
//! precedent of never hand-rolling something the system already does
//! correctly. The ICMP packet itself, though, has no `std` support at
//! all, so building and parsing it is genuinely this module's job
//! (see `build_echo_request`/`checksum` below, verified against a
//! from-scratch Python re-implementation before being written here).

use crate::common::errors::{AppError, AppResult};
use std::io;
use std::net::Ipv4Addr;
use std::os::raw::{c_int, c_void};
use std::time::{Duration, Instant};

pub const USAGE: &str = "ping [-c COUNT] HOST -- send ICMPv4 echo requests (default count 4)";

const AF_INET: c_int = 2;
const SOCK_DGRAM: c_int = 2;
const SOCK_RAW: c_int = 3;
const IPPROTO_ICMP: c_int = 1;
const SOL_SOCKET: c_int = 1;
const SO_RCVTIMEO: c_int = 20;
const ICMP_ECHO_REPLY: u8 = 0;
const ICMP_ECHO_REQUEST: u8 = 8;
const REPLY_TIMEOUT: Duration = Duration::from_secs(1);
const PING_INTERVAL: Duration = Duration::from_secs(1);
const PAYLOAD_LEN: usize = 32;

mod ffi {
    use super::*;

    // Layout matches glibc's `struct sockaddr_in` on Linux (see
    // `<netinet/in.h>`): sin_family and sin_port are host-order u16s,
    // sin_zero is padding to match `struct sockaddr`'s size. sin_addr
    // is deliberately a plain 4-byte array here, not a u32 -- that
    // sidesteps host-endianness entirely, since `Ipv4Addr::octets()`
    // already returns the bytes in the order the kernel wants them,
    // and copying an array byte-for-byte can't get that backwards the
    // way constructing an integer and reasoning about its byte order
    // could.
    #[repr(C)]
    pub struct SockaddrIn {
        pub sin_family: u16,
        pub sin_port: u16,
        pub sin_addr: [u8; 4],
        pub sin_zero: [u8; 8],
    }

    // Layout matches glibc's `struct timeval` on 64-bit Linux
    // (x86_64 and aarch64 both use 64-bit time_t/suseconds_t).
    #[repr(C)]
    pub struct Timeval {
        pub tv_sec: i64,
        pub tv_usec: i64,
    }

    extern "C" {
        pub fn socket(domain: c_int, ty: c_int, protocol: c_int) -> c_int;
        pub fn close(fd: c_int) -> c_int;
        pub fn sendto(
            fd: c_int,
            buf: *const c_void,
            len: usize,
            flags: c_int,
            dest_addr: *const SockaddrIn,
            addrlen: u32,
        ) -> isize;
        pub fn recvfrom(
            fd: c_int,
            buf: *mut c_void,
            len: usize,
            flags: c_int,
            src_addr: *mut SockaddrIn,
            addrlen: *mut u32,
        ) -> isize;
        pub fn setsockopt(
            fd: c_int,
            level: c_int,
            optname: c_int,
            optval: *const c_void,
            optlen: u32,
        ) -> c_int;
    }
}

/// Owns a raw socket fd and closes it on drop, so an early return (a
/// resolution failure, a bad argument) can't leak it.
struct RawSocket(c_int);

impl Drop for RawSocket {
    fn drop(&mut self) {
        unsafe { ffi::close(self.0) };
    }
}

/// The standard Internet checksum (RFC 1071): sum every 16-bit word
/// as ones-complement addition (carries wrap back around, handled
/// here by folding the high 16 bits back in after the fact rather
/// than during, which gives the same result), one extra zero-padded
/// byte if the length is odd, then complement the total. Verified
/// against an independent Python implementation across both an
/// even-length and an odd-length packet before being written here --
/// inserting the checksum this returns and re-summing the whole
/// packet must reduce to exactly 0, which is exactly what that check
/// confirmed.
fn checksum(data: &[u8]) -> u16 {
    let mut sum: u32 = 0;
    let mut chunks = data.chunks_exact(2);
    for chunk in &mut chunks {
        sum = sum.wrapping_add(u16::from_be_bytes([chunk[0], chunk[1]]) as u32);
    }
    if let [last] = chunks.remainder() {
        sum = sum.wrapping_add((*last as u32) << 8);
    }
    while sum >> 16 != 0 {
        sum = (sum & 0xFFFF) + (sum >> 16);
    }
    !(sum as u16)
}

fn build_echo_request(identifier: u16, sequence: u16, payload: &[u8]) -> Vec<u8> {
    let mut packet = Vec::with_capacity(8 + payload.len());
    packet.push(ICMP_ECHO_REQUEST);
    packet.push(0); // code
    packet.push(0); // checksum, filled in below
    packet.push(0);
    packet.extend_from_slice(&identifier.to_be_bytes());
    packet.extend_from_slice(&sequence.to_be_bytes());
    packet.extend_from_slice(payload);

    let sum = checksum(&packet).to_be_bytes();
    packet[2] = sum[0];
    packet[3] = sum[1];
    packet
}

fn resolve_ipv4(host: &str) -> AppResult<Ipv4Addr> {
    use std::net::ToSocketAddrs;
    // ToSocketAddrs needs a "host:port" pair even though ICMP has no
    // port concept -- the 0 is discarded immediately below.
    let addrs = format!("{host}:0")
        .to_socket_addrs()
        .map_err(|e| AppError::new(format!("cannot resolve '{host}': {e}")))?;
    addrs
        .filter_map(|a| match a {
            std::net::SocketAddr::V4(v4) => Some(*v4.ip()),
            std::net::SocketAddr::V6(_) => None,
        })
        .next()
        .ok_or_else(|| AppError::new(format!("'{host}' has no IPv4 address (IPv6 targets aren't supported yet)")))
}

/// Tries the unprivileged Linux ping-socket first, falls back to a
/// raw socket (needs root/CAP_NET_RAW) if that's refused. The `bool`
/// says which one it got: `true` = ping-socket (replies arrive as a
/// bare ICMP message), `false` = raw (replies arrive with a full IP
/// header in front that the caller has to skip).
fn open_socket() -> AppResult<(RawSocket, bool)> {
    let fd = unsafe { ffi::socket(AF_INET, SOCK_DGRAM, IPPROTO_ICMP) };
    if fd >= 0 {
        return Ok((RawSocket(fd), true));
    }
    let fd = unsafe { ffi::socket(AF_INET, SOCK_RAW, IPPROTO_ICMP) };
    if fd >= 0 {
        return Ok((RawSocket(fd), false));
    }
    Err(AppError::new(format!(
        "cannot open an ICMP socket ({}) -- try running as root, or check \
         net.ipv4.ping_group_range allows this user",
        io::Error::last_os_error()
    )))
}

fn set_recv_timeout(sock: &RawSocket, timeout: Duration) -> io::Result<()> {
    let tv = ffi::Timeval {
        tv_sec: timeout.as_secs() as i64,
        tv_usec: timeout.subsec_micros() as i64,
    };
    let rc = unsafe {
        ffi::setsockopt(
            sock.0,
            SOL_SOCKET,
            SO_RCVTIMEO,
            &tv as *const _ as *const c_void,
            std::mem::size_of::<ffi::Timeval>() as u32,
        )
    };
    if rc == 0 {
        Ok(())
    } else {
        Err(io::Error::last_os_error())
    }
}

fn send_echo(sock: &RawSocket, dest: Ipv4Addr, identifier: u16, sequence: u16) -> io::Result<()> {
    let payload = [0u8; PAYLOAD_LEN];
    let packet = build_echo_request(identifier, sequence, &payload);
    let addr = ffi::SockaddrIn {
        sin_family: AF_INET as u16,
        sin_port: 0, // ICMP has no ports
        sin_addr: dest.octets(),
        sin_zero: [0; 8],
    };
    let sent = unsafe {
        ffi::sendto(
            sock.0,
            packet.as_ptr() as *const c_void,
            packet.len(),
            0,
            &addr,
            std::mem::size_of::<ffi::SockaddrIn>() as u32,
        )
    };
    if sent < 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}

/// Waits (up to `REPLY_TIMEOUT`, set on the socket) for a reply that
/// looks like it actually answers `(identifier, sequence)`. On a raw
/// socket, everything else the kernel would otherwise deliver here
/// too -- another program's ping, some other ICMP message type -- is
/// silently skipped rather than misreported as this one's reply.
fn receive_reply(sock: &RawSocket, is_ping_socket: bool, identifier: u16, sequence: u16) -> Option<()> {
    let mut buf = [0u8; 128];
    loop {
        let received = unsafe {
            ffi::recvfrom(
                sock.0,
                buf.as_mut_ptr() as *mut c_void,
                buf.len(),
                0,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            )
        };
        if received < 0 {
            return None; // timeout (EAGAIN/EWOULDBLOCK) or another error
        }
        let received = received as usize;

        // A raw socket includes the IP header; a ping-socket doesn't.
        let icmp_start = if is_ping_socket {
            0
        } else {
            if received == 0 {
                continue;
            }
            ((buf[0] & 0x0F) as usize) * 4
        };
        if received < icmp_start + 8 {
            continue; // too short to be a real ICMP message
        }
        let icmp = &buf[icmp_start..received];
        let reply_seq = u16::from_be_bytes([icmp[6], icmp[7]]);
        let matches = if is_ping_socket {
            // The kernel only delivers replies addressed to *this*
            // ping-socket in the first place, so sequence number
            // alone is enough here -- whether it also preserves this
            // process's exact identifier value on the wire isn't
            // something this relies on either way.
            icmp[0] == ICMP_ECHO_REPLY && reply_seq == sequence
        } else {
            let reply_id = u16::from_be_bytes([icmp[4], icmp[5]]);
            icmp[0] == ICMP_ECHO_REPLY && reply_id == identifier && reply_seq == sequence
        };
        if matches {
            return Some(());
        }
        // Something else arrived on this socket -- keep waiting until
        // the timeout, rather than treating it as this ping's answer.
    }
}

pub fn run(args: Vec<String>) -> AppResult<()> {
    let (opts, forced) = crate::common::args::split_dashdash(args);
    let mut count: u32 = 4;
    let mut host: Option<String> = None;

    let mut iter = opts.into_iter();
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "-c" => {
                count = iter
                    .next()
                    .ok_or_else(|| AppError::usage("-c requires an argument"))?
                    .parse()
                    .map_err(|_| AppError::usage("-c requires a positive integer"))?;
            }
            other if other.starts_with('-') => {
                return Err(AppError::usage(format!("unknown option '{other}'")));
            }
            other if host.is_none() => host = Some(other.to_string()),
            other => return Err(AppError::usage(format!("unexpected argument '{other}'"))),
        }
    }
    if host.is_none() {
        host = forced.into_iter().next();
    }
    let host = host.ok_or_else(|| AppError::usage("missing host"))?;
    if count == 0 {
        return Err(AppError::usage("-c must be at least 1"));
    }

    let dest = resolve_ipv4(&host)?;
    let (sock, is_ping_socket) = open_socket()?;
    set_recv_timeout(&sock, REPLY_TIMEOUT)
        .map_err(|e| AppError::new(format!("cannot set receive timeout: {e}")))?;

    println!("PING {host} ({dest}): {PAYLOAD_LEN} data bytes");
    let identifier = (std::process::id() & 0xFFFF) as u16;
    let mut sent = 0u32;
    let mut received = 0u32;
    let mut rtts: Vec<Duration> = Vec::new();

    for sequence in 0..count {
        sent += 1;
        let started = Instant::now();
        if let Err(e) = send_echo(&sock, dest, identifier, sequence as u16) {
            eprintln!("ping: send failed: {e}");
            continue;
        }
        match receive_reply(&sock, is_ping_socket, identifier, sequence as u16) {
            Some(()) => {
                let rtt = started.elapsed();
                received += 1;
                rtts.push(rtt);
                println!(
                    "{PAYLOAD_LEN} bytes from {dest}: icmp_seq={sequence} time={:.1} ms",
                    rtt.as_secs_f64() * 1000.0
                );
            }
            None => println!("Request timeout for icmp_seq={sequence}"),
        }
        if sequence + 1 < count {
            std::thread::sleep(PING_INTERVAL);
        }
    }

    let loss_pct = if sent == 0 {
        0.0
    } else {
        100.0 * (sent - received) as f64 / sent as f64
    };
    println!("\n--- {host} ping statistics ---");
    println!(
        "{sent} packets transmitted, {received} received, {loss_pct:.0}% packet loss"
    );
    if !rtts.is_empty() {
        let millis: Vec<f64> = rtts.iter().map(|d| d.as_secs_f64() * 1000.0).collect();
        let min = millis.iter().cloned().fold(f64::INFINITY, f64::min);
        let max = millis.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let avg = millis.iter().sum::<f64>() / millis.len() as f64;
        println!("round-trip min/avg/max = {min:.1}/{avg:.1}/{max:.1} ms");
    }

    if received == 0 {
        Err(AppError::silent(1))
    } else {
        Ok(())
    }
}
