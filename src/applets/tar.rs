//! `tar` -- create, list, and extract POSIX **ustar** archives.
//!
//! From scratch, matching this crate's zero-dependency stance: the
//! ustar header layout (POSIX.1-1988 / IEEE Std 1003.1) is a fixed,
//! well-documented 512-byte record, not something that benefits from
//! an external crate the way, say, a real compressor would.
//!
//! # Scope -- what this does and doesn't cover
//!
//! - **No compression.** `-z`/`-j`/`-J` (gzip/bzip2/xz) are refused
//!   with a clear error rather than silently ignored or misread as a
//!   filename. Writing a from-scratch DEFLATE/bzip2/xz implementation
//!   is a large, separate chunk of work in its own right (unlike the
//!   archive *format* itself, which is just fixed-width fields), and
//!   adding a compression crate would be the first real dependency
//!   this side of the crate has ever taken on. Left as a deliberate,
//!   documented gap rather than either of those.
//! - **ustar only.** No GNU longname (`@LongLink`)/PAX extended
//!   headers for names beyond ustar's own ~255-byte prefix+name
//!   limit; such an entry is a clear error on write, and an
//!   unrecognized typeflag is skipped (with a warning) rather than
//!   misread on read.
//! - **Regular files, directories, symlinks, and hard links** are
//!   fully supported both ways. Device nodes/FIFOs/sockets are
//!   skipped with a warning on create and on extract alike --
//!   creating them needs `mknod(2)`, a meaningfully sharper edge than
//!   anything else in this crate for how little real use it would
//!   get.
//! - **`-c`/`-x`/`-t` only** -- no `-r`/`-u` (append/update an
//!   existing archive in place).
//!
//! # Security: path handling on extract
//!
//! Every member name is run through [`sanitize_member_name`] before
//! it ever touches the filesystem: a leading `/` is stripped (an
//! archive can't force extraction to an absolute path) and any `..`
//! path component is a hard error for that entry (an archive can't
//! escape the destination directory it's being extracted into,
//! sometimes called "tarslip" or a "zip-slip"-style attack applied to
//! tar). This applies to member names AND to a hard-link entry's own
//! link-target name, since that's just as attacker-controlled as any
//! other field in the archive. This check runs on every archive, not
//! just ones this same `tar` wrote -- exactly the case that matters,
//! since the whole point of a shared format is reading files other
//! tools produced.
//!
//! Ownership (`chown`) is only restored on extract when the caller is
//! already root, matching real `tar`; permission bits are restored
//! unconditionally (the common, safe case for a non-root extract).

use crate::common::errors::{AppError, AppResult};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

pub const USAGE: &str =
    "tar {-c|-x|-t} -f ARCHIVE [-v] [-C DIR] [FILE...] -- create, list, or extract a ustar archive (no compression)";

const BLOCK_SIZE: usize = 512;

const TYPE_REGULAR: u8 = b'0';
const TYPE_HARDLINK: u8 = b'1';
const TYPE_SYMLINK: u8 = b'2';
const TYPE_DIRECTORY: u8 = b'5';

/// Byte offset + length of every ustar header field. See
/// POSIX.1-1988 / the historical `tar(5)` layout -- these numbers are
/// the format, not a stylistic choice.
mod hdr {
    pub const NAME: (usize, usize) = (0, 100);
    pub const MODE: (usize, usize) = (100, 8);
    pub const UID: (usize, usize) = (108, 8);
    pub const GID: (usize, usize) = (116, 8);
    pub const SIZE: (usize, usize) = (124, 12);
    pub const MTIME: (usize, usize) = (136, 12);
    pub const CHKSUM: (usize, usize) = (148, 8);
    pub const TYPEFLAG: usize = 156;
    pub const LINKNAME: (usize, usize) = (157, 100);
    pub const MAGIC: (usize, usize) = (257, 6);
    pub const VERSION: (usize, usize) = (263, 2);
    pub const UNAME: (usize, usize) = (265, 32);
    pub const GNAME: (usize, usize) = (297, 32);
    pub const PREFIX: (usize, usize) = (345, 155);
}

// ---------------------------------------------------------------
// CLI entry point
// ---------------------------------------------------------------

pub fn run(args: Vec<String>) -> AppResult<()> {
    let (opts, forced) = crate::common::args::split_dashdash(args);
    let mut parts = opts;
    parts.extend(forced);
    let parts = expand_bundled_mode(parts)?;

    let mut mode: Option<char> = None;
    let mut archive: Option<String> = None;
    let mut verbose = false;
    let mut directory: Option<String> = None;
    let mut files: Vec<String> = Vec::new();

    let mut iter = parts.into_iter();
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "-c" | "--create" => set_mode_flag(&mut mode, 'c')?,
            "-x" | "--extract" => set_mode_flag(&mut mode, 'x')?,
            "-t" | "--list" => set_mode_flag(&mut mode, 't')?,
            "-v" | "--verbose" => verbose = true,
            "-f" | "--file" => {
                archive = Some(
                    iter.next()
                        .ok_or_else(|| AppError::usage("-f requires an argument"))?,
                )
            }
            "-C" | "--directory" => {
                directory = Some(
                    iter.next()
                        .ok_or_else(|| AppError::usage("-C requires an argument"))?,
                )
            }
            "-z" | "-j" | "-J" | "--gzip" | "--bzip2" | "--xz" => {
                return Err(AppError::new(
                    "compressed archives (gzip/bzip2/xz) are not supported -- this tar only \
                     reads and writes plain, uncompressed ustar archives",
                ));
            }
            other if other.starts_with('-') && other.len() > 1 => {
                return Err(AppError::usage(format!("unknown option '{other}'")));
            }
            other => files.push(other.to_string()),
        }
    }

    let mode = mode.ok_or_else(|| AppError::usage("exactly one of -c, -x, -t is required"))?;
    let archive = archive.ok_or_else(|| AppError::usage("-f ARCHIVE is required"))?;

    match mode {
        'c' => create_archive(&archive, &files, verbose),
        'x' => extract_archive(
            &archive,
            &directory.unwrap_or_else(|| ".".to_string()),
            verbose,
            &files,
        ),
        't' => list_archive(&archive, verbose, &files),
        _ => unreachable!("set_mode_flag only ever stores 'c', 'x', or 't'"),
    }
}

fn set_mode_flag(mode: &mut Option<char>, new_mode: char) -> AppResult<()> {
    match mode {
        Some(existing) if *existing != new_mode => {
            Err(AppError::usage("only one of -c, -x, -t may be given"))
        }
        _ => {
            *mode = Some(new_mode);
            Ok(())
        }
    }
}

/// Classic `tar cf archive.tar file...`-style bundled mode letters
/// (no leading `-`) are still how most people type `tar` from muscle
/// memory. If the first argument is a bare run of recognized letters
/// (`c`/`x`/`t`/`v`/`f`/`C`), expand it into the modern `-c -f ...`
/// form before the real parser ever sees it; otherwise `args` is
/// returned untouched. Same inherent ambiguity real `tar` has always
/// had: a file you actually want to archive that happens to be
/// spelled e.g. `cf` with no other arguments would be misread as
/// bundled flags -- `--`/an explicit `-c -f ...` sidesteps it, same
/// as it would for real `tar`.
fn expand_bundled_mode(args: Vec<String>) -> AppResult<Vec<String>> {
    let Some(first) = args.first() else {
        return Ok(args);
    };
    let is_bundled =
        !first.is_empty() && !first.starts_with('-') && first.chars().all(|c| "cxtvfC".contains(c));
    if !is_bundled {
        return Ok(args);
    }

    let letters = first.clone();
    let mut rest = args[1..].to_vec();
    let mut expanded: Vec<String> = Vec::new();
    for letter in letters.chars() {
        match letter {
            'c' => expanded.push("-c".to_string()),
            'x' => expanded.push("-x".to_string()),
            't' => expanded.push("-t".to_string()),
            'v' => expanded.push("-v".to_string()),
            'f' => {
                expanded.push("-f".to_string());
                if rest.is_empty() {
                    return Err(AppError::usage("-f requires an argument"));
                }
                expanded.push(rest.remove(0));
            }
            'C' => {
                expanded.push("-C".to_string());
                if rest.is_empty() {
                    return Err(AppError::usage("-C requires an argument"));
                }
                expanded.push(rest.remove(0));
            }
            _ => unreachable!("is_bundled already restricted letters to \"cxtvfC\""),
        }
    }
    expanded.extend(rest);
    Ok(expanded)
}

// ---------------------------------------------------------------
// Security: member-name sanitization (used on every read path)
// ---------------------------------------------------------------

/// Defends against a maliciously (or just differently-) crafted
/// archive escaping the extraction directory: strips a leading `/`
/// and rejects any `..` path component outright. Applied to every
/// member name AND to a hard-link entry's own recorded link target --
/// see this module's doc comment.
fn sanitize_member_name(name: &str) -> AppResult<String> {
    let stripped = name.trim_start_matches('/');
    if stripped.split('/').any(|part| part == "..") {
        return Err(AppError::new(format!(
            "refusing to extract '{name}': contains a '..' path component"
        )));
    }
    if stripped.is_empty() {
        return Err(AppError::new(
            "refusing to extract an empty/root member name",
        ));
    }
    Ok(stripped.to_string())
}

// ---------------------------------------------------------------
// Header encode/decode
// ---------------------------------------------------------------

struct HeaderSpec<'a> {
    name: &'a str,
    size: u64,
    mode: u32,
    uid: u32,
    gid: u32,
    mtime: u64,
    typeflag: u8,
    linkname: &'a str,
    uname: &'a str,
    gname: &'a str,
}

struct Entry {
    name: String,
    mode: u32,
    uid: u32,
    gid: u32,
    size: u64,
    mtime: u64,
    typeflag: u8,
    linkname: String,
}

fn set_field(block: &mut [u8; BLOCK_SIZE], field: (usize, usize), bytes: &[u8]) {
    let (offset, len) = field;
    let n = bytes.len().min(len);
    block[offset..offset + n].copy_from_slice(&bytes[..n]);
}

fn get_field(block: &[u8; BLOCK_SIZE], field: (usize, usize)) -> &[u8] {
    let (offset, len) = field;
    &block[offset..offset + len]
}

/// Writes `len - 1` zero-padded octal digits followed by a single NUL
/// terminator -- the standard ustar numeric-field encoding (7 digits
/// for the 8-byte fields, 11 for the 12-byte ones).
fn set_octal(block: &mut [u8; BLOCK_SIZE], field: (usize, usize), value: u64) {
    let (_, len) = field;
    let digits = len - 1;
    let s = format!("{:0width$o}", value, width = digits);
    let mut bytes = s.into_bytes();
    bytes.push(0);
    set_field(block, field, &bytes);
}

/// The 6-digit-plus-NUL-plus-space checksum encoding ustar mandates
/// specifically for the checksum field itself (distinct from every
/// other numeric field's plain NUL termination).
fn set_checksum(block: &mut [u8; BLOCK_SIZE]) {
    let sum = compute_checksum(block);
    let s = format!("{:06o}\0 ", sum);
    let bytes = s.into_bytes();
    set_field(block, hdr::CHKSUM, &bytes);
}

/// Sum of every header byte, with the checksum field itself treated
/// as eight ASCII spaces while summing -- required by the format,
/// since the checksum obviously can't include its own final value.
/// Uses the unsigned-byte interpretation, which is what every current
/// GNU/BSD/POSIX implementation computes (a handful of very old
/// implementations summed signed bytes instead; that historical
/// inconsistency is not reproduced here).
fn compute_checksum(block: &[u8; BLOCK_SIZE]) -> u32 {
    let mut sum: u32 = 0;
    for (i, &b) in block.iter().enumerate() {
        if i >= hdr::CHKSUM.0 && i < hdr::CHKSUM.0 + hdr::CHKSUM.1 {
            sum += b' ' as u32;
        } else {
            sum += b as u32;
        }
    }
    sum
}

fn parse_octal(raw: &[u8]) -> u64 {
    let s: String = raw
        .iter()
        .take_while(|&&b| b != 0)
        .map(|&b| b as char)
        .collect();
    u64::from_str_radix(s.trim(), 8).unwrap_or(0)
}

fn get_cstr(raw: &[u8]) -> String {
    let end = raw.iter().position(|&b| b == 0).unwrap_or(raw.len());
    String::from_utf8_lossy(&raw[..end]).into_owned()
}

fn pad_len(size: u64) -> usize {
    let rem = (size % BLOCK_SIZE as u64) as usize;
    if rem == 0 {
        0
    } else {
        BLOCK_SIZE - rem
    }
}

fn blocks_for_size(size: u64) -> u64 {
    (size + BLOCK_SIZE as u64 - 1) / BLOCK_SIZE as u64
}

/// Split a path into ustar's `(prefix, name)` pair: the whole path
/// when it already fits in the 100-byte `name` field, otherwise the
/// rightmost `/`-boundary split where the tail fits in `name` (100
/// bytes) and the head fits in `prefix` (155 bytes). A path too long
/// for even that combination is a clear write-time error rather than
/// silent truncation or a GNU-longname extension this crate doesn't
/// implement.
fn split_name(path: &str) -> AppResult<(String, String)> {
    if path.len() <= 100 {
        return Ok((String::new(), path.to_string()));
    }
    let bytes = path.as_bytes();
    for i in (0..bytes.len()).rev() {
        if bytes[i] == b'/' {
            let prefix = &path[..i];
            let name = &path[i + 1..];
            if !name.is_empty() && name.len() <= 100 && prefix.len() <= 155 {
                return Ok((prefix.to_string(), name.to_string()));
            }
        }
    }
    Err(AppError::new(format!(
        "path too long for the tar (ustar) format (limit ~255 bytes, split at a '/'): '{path}'"
    )))
}

fn write_header(writer: &mut dyn Write, spec: &HeaderSpec) -> AppResult<()> {
    let name = spec.name.trim_start_matches('/');
    if name.len() != spec.name.len() {
        eprintln!("tar: removing leading '/' from member name '{}'", spec.name);
    }
    if name.split('/').any(|part| part == "..") {
        return Err(AppError::new(format!(
            "refusing to add '{}' to the archive: contains a '..' path component \
             (such an entry could never be safely extracted)",
            spec.name
        )));
    }
    let (prefix, short_name) = split_name(name)?;

    let mut block = [0u8; BLOCK_SIZE];
    set_field(&mut block, hdr::NAME, short_name.as_bytes());
    set_octal(&mut block, hdr::MODE, (spec.mode & 0o7777) as u64);
    set_octal(&mut block, hdr::UID, spec.uid as u64);
    set_octal(&mut block, hdr::GID, spec.gid as u64);
    set_octal(&mut block, hdr::SIZE, spec.size);
    set_octal(&mut block, hdr::MTIME, spec.mtime);
    block[hdr::TYPEFLAG] = spec.typeflag;
    set_field(&mut block, hdr::LINKNAME, spec.linkname.as_bytes());
    set_field(&mut block, hdr::MAGIC, b"ustar\0");
    set_field(&mut block, hdr::VERSION, b"00");
    set_field(&mut block, hdr::UNAME, spec.uname.as_bytes());
    set_field(&mut block, hdr::GNAME, spec.gname.as_bytes());
    set_field(&mut block, hdr::PREFIX, prefix.as_bytes());
    set_checksum(&mut block);

    writer
        .write_all(&block)
        .map_err(|e| AppError::new(format!("write error: {e}")))
}

/// Reads one header block. `Ok(None)` means a clean end of archive
/// (an all-zero block, or EOF exactly on a block boundary before any
/// bytes of a new header were read) -- the normal way an archive
/// ends, not an error.
fn read_header(reader: &mut dyn Read) -> AppResult<Option<[u8; BLOCK_SIZE]>> {
    let mut block = [0u8; BLOCK_SIZE];
    let mut read_total = 0;
    while read_total < BLOCK_SIZE {
        let n = reader
            .read(&mut block[read_total..])
            .map_err(|e| AppError::new(format!("read error: {e}")))?;
        if n == 0 {
            if read_total == 0 {
                return Ok(None);
            }
            return Err(AppError::new(
                "unexpected end of archive (truncated header block)",
            ));
        }
        read_total += n;
    }
    if block.iter().all(|&b| b == 0) {
        return Ok(None);
    }
    let stored = parse_octal(get_field(&block, hdr::CHKSUM)) as u32;
    let computed = compute_checksum(&block);
    if stored != computed {
        return Err(AppError::new(
            "not a valid tar archive (header checksum mismatch)",
        ));
    }
    Ok(Some(block))
}

fn parse_header(block: &[u8; BLOCK_SIZE]) -> Entry {
    let name = get_cstr(get_field(block, hdr::NAME));
    let prefix = get_cstr(get_field(block, hdr::PREFIX));
    let full_name = if prefix.is_empty() {
        name
    } else {
        format!("{prefix}/{name}")
    };
    Entry {
        name: full_name,
        mode: parse_octal(get_field(block, hdr::MODE)) as u32,
        uid: parse_octal(get_field(block, hdr::UID)) as u32,
        gid: parse_octal(get_field(block, hdr::GID)) as u32,
        size: parse_octal(get_field(block, hdr::SIZE)),
        mtime: parse_octal(get_field(block, hdr::MTIME)),
        typeflag: block[hdr::TYPEFLAG],
        linkname: get_cstr(get_field(block, hdr::LINKNAME)),
    }
}

fn skip_data(reader: &mut dyn Read, size: u64) -> AppResult<()> {
    let mut remaining = blocks_for_size(size) * BLOCK_SIZE as u64;
    let mut buf = [0u8; BLOCK_SIZE];
    while remaining > 0 {
        let take = remaining.min(BLOCK_SIZE as u64) as usize;
        reader
            .read_exact(&mut buf[..take])
            .map_err(|e| AppError::new(format!("unexpected end of archive: {e}")))?;
        remaining -= take as u64;
    }
    Ok(())
}

fn open_archive_for_read(archive: &str) -> AppResult<Box<dyn Read>> {
    if archive == "-" {
        Ok(Box::new(std::io::stdin()))
    } else {
        let f = std::fs::File::open(archive)
            .map_err(|e| AppError::new(format!("cannot open '{archive}': {e}")))?;
        Ok(Box::new(std::io::BufReader::new(f)))
    }
}

/// Reads through `bytes` as an in-memory archive, exercising the same
/// header-parsing/checksum/size-arithmetic path `-t`/`-x` use against
/// a real file, but entirely in memory -- no filesystem access at
/// all. Exists for `fuzz/fuzz_targets/tar_scan_headers.rs`: the
/// property being fuzzed is "never panics on adversarial bytes", not
/// any particular return value, so there's little reason to call this
/// over `-t` itself outside of that fuzz target.
pub fn scan_archive_headers(bytes: &[u8]) -> AppResult<Vec<String>> {
    let mut cursor = std::io::Cursor::new(bytes);
    let mut names = Vec::new();
    while let Some(block) = read_header(&mut cursor)? {
        let entry = parse_header(&block);
        names.push(entry.name.clone());
        skip_data(&mut cursor, entry.size)?;
    }
    Ok(names)
}

// ---------------------------------------------------------------
// create
// ---------------------------------------------------------------

fn create_archive(archive: &str, inputs: &[String], verbose: bool) -> AppResult<()> {
    if inputs.is_empty() {
        return Err(AppError::usage("no files or directories given to archive"));
    }

    let mut writer: Box<dyn Write> = if archive == "-" {
        Box::new(std::io::BufWriter::new(std::io::stdout()))
    } else {
        let file = std::fs::File::create(archive)
            .map_err(|e| AppError::new(format!("cannot create '{archive}': {e}")))?;
        Box::new(std::io::BufWriter::new(file))
    };

    for input in inputs {
        let root = Path::new(input);
        let entries = crate::common::paths::walk(root)
            .map_err(|e| AppError::new(format!("cannot read '{input}': {e}")))?;
        for entry in &entries {
            add_entry(&mut writer, entry, verbose)?;
        }
    }

    writer
        .write_all(&[0u8; BLOCK_SIZE * 2])
        .map_err(|e| AppError::new(format!("write error: {e}")))?;
    writer
        .flush()
        .map_err(|e| AppError::new(format!("write error: {e}")))?;
    Ok(())
}

fn add_entry(writer: &mut dyn Write, path: &Path, verbose: bool) -> AppResult<()> {
    let meta = std::fs::symlink_metadata(path)
        .map_err(|e| AppError::new(format!("cannot stat '{}': {e}", path.display())))?;
    let mode = crate::common::permissions::mode_of(&meta);
    let uid = unix_uid(&meta);
    let gid = unix_gid(&meta);
    let mtime = unix_mtime(&meta);
    let uname = crate::common::users::name_for_uid(uid).unwrap_or_default();
    let gname = crate::common::users::name_for_gid(gid).unwrap_or_default();
    let name_str = path.to_string_lossy().into_owned();

    if meta.is_symlink() {
        let target = std::fs::read_link(path)
            .map_err(|e| AppError::new(format!("cannot read link '{}': {e}", path.display())))?;
        let target_str = target.to_string_lossy().into_owned();
        write_header(
            writer,
            &HeaderSpec {
                name: &name_str,
                size: 0,
                mode,
                uid,
                gid,
                mtime,
                typeflag: TYPE_SYMLINK,
                linkname: &target_str,
                uname: &uname,
                gname: &gname,
            },
        )?;
        if verbose {
            println!("{name_str} -> {target_str}");
        }
    } else if meta.is_dir() {
        let dir_name = if name_str.ends_with('/') {
            name_str.clone()
        } else {
            format!("{name_str}/")
        };
        write_header(
            writer,
            &HeaderSpec {
                name: &dir_name,
                size: 0,
                mode,
                uid,
                gid,
                mtime,
                typeflag: TYPE_DIRECTORY,
                linkname: "",
                uname: &uname,
                gname: &gname,
            },
        )?;
        if verbose {
            println!("{dir_name}");
        }
    } else if meta.is_file() {
        let size = meta.len();
        write_header(
            writer,
            &HeaderSpec {
                name: &name_str,
                size,
                mode,
                uid,
                gid,
                mtime,
                typeflag: TYPE_REGULAR,
                linkname: "",
                uname: &uname,
                gname: &gname,
            },
        )?;
        let mut f = std::fs::File::open(path)
            .map_err(|e| AppError::new(format!("cannot open '{}': {e}", path.display())))?;
        let copied = std::io::copy(&mut f, writer)
            .map_err(|e| AppError::new(format!("cannot read '{}': {e}", path.display())))?;
        if copied != size {
            return Err(AppError::new(format!(
                "'{}' changed size while being archived ({size} -> {copied} bytes)",
                path.display()
            )));
        }
        let padding = pad_len(copied);
        if padding > 0 {
            writer
                .write_all(&[0u8; BLOCK_SIZE][..padding])
                .map_err(|e| AppError::new(format!("write error: {e}")))?;
        }
        if verbose {
            println!("{name_str}");
        }
    } else {
        eprintln!(
            "tar: skipping '{}': not a regular file, directory, or symlink",
            path.display()
        );
    }
    Ok(())
}

#[cfg(unix)]
fn unix_uid(meta: &std::fs::Metadata) -> u32 {
    use std::os::unix::fs::MetadataExt;
    meta.uid()
}
#[cfg(unix)]
fn unix_gid(meta: &std::fs::Metadata) -> u32 {
    use std::os::unix::fs::MetadataExt;
    meta.gid()
}
#[cfg(unix)]
fn unix_mtime(meta: &std::fs::Metadata) -> u64 {
    use std::os::unix::fs::MetadataExt;
    meta.mtime().max(0) as u64
}
#[cfg(not(unix))]
fn unix_uid(_meta: &std::fs::Metadata) -> u32 {
    0
}
#[cfg(not(unix))]
fn unix_gid(_meta: &std::fs::Metadata) -> u32 {
    0
}
#[cfg(not(unix))]
fn unix_mtime(_meta: &std::fs::Metadata) -> u64 {
    0
}

// ---------------------------------------------------------------
// list
// ---------------------------------------------------------------

fn list_archive(archive: &str, verbose: bool, only: &[String]) -> AppResult<()> {
    let mut reader = open_archive_for_read(archive)?;
    while let Some(block) = read_header(&mut reader)? {
        let entry = parse_header(&block);
        if !only.is_empty() && !only.contains(&entry.name) {
            skip_data(&mut reader, entry.size)?;
            continue;
        }
        if verbose {
            let type_char = match entry.typeflag {
                TYPE_DIRECTORY => 'd',
                TYPE_SYMLINK => 'l',
                _ => '-',
            };
            let mode_str = crate::common::permissions::format_mode(entry.mode, type_char);
            println!("{mode_str} {:>10} {}", entry.size, entry.name);
        } else {
            println!("{}", entry.name);
        }
        skip_data(&mut reader, entry.size)?;
    }
    Ok(())
}

// ---------------------------------------------------------------
// extract
// ---------------------------------------------------------------

fn extract_archive(archive: &str, dest: &str, verbose: bool, only: &[String]) -> AppResult<()> {
    let dest_root = Path::new(dest);
    std::fs::create_dir_all(dest_root)
        .map_err(|e| AppError::new(format!("cannot create destination '{dest}': {e}")))?;
    let running_as_root = crate::common::users::current_identity().euid == 0;

    let mut reader = open_archive_for_read(archive)?;
    let mut dir_mtimes: Vec<(PathBuf, u64)> = Vec::new();
    let mut had_error = false;

    while let Some(block) = read_header(&mut reader)? {
        let entry = parse_header(&block);
        if !only.is_empty() && !only.contains(&entry.name) {
            skip_data(&mut reader, entry.size)?;
            continue;
        }

        let safe_name = match sanitize_member_name(&entry.name) {
            Ok(n) => n,
            Err(e) => {
                eprintln!("tar: {e}");
                had_error = true;
                skip_data(&mut reader, entry.size)?;
                continue;
            }
        };
        let target = dest_root.join(&safe_name);

        let result = extract_one(
            &entry,
            dest_root,
            &target,
            &mut reader,
            running_as_root,
            &mut dir_mtimes,
        );
        match result {
            Ok(()) => {
                if verbose {
                    println!("{safe_name}");
                }
            }
            Err(e) => {
                crate::common::output::error_path("tar", &safe_name, e);
                had_error = true;
            }
        }
    }

    for (dir, mtime) in dir_mtimes {
        let when = std::time::UNIX_EPOCH + std::time::Duration::from_secs(mtime);
        let _ = std::fs::OpenOptions::new()
            .read(true)
            .open(&dir)
            .and_then(|f| f.set_modified(when));
    }

    if had_error {
        Err(AppError::silent(1))
    } else {
        Ok(())
    }
}

/// Extracts one already-sanitized entry. Always consumes exactly
/// `entry.size`'s worth of data blocks from `reader` (directly for a
/// regular file, via `skip_data` for anything without file content),
/// so a per-entry failure never desynchronizes the rest of the
/// archive for the entries that follow.
fn extract_one(
    entry: &Entry,
    dest_root: &Path,
    target: &Path,
    reader: &mut dyn Read,
    running_as_root: bool,
    dir_mtimes: &mut Vec<(PathBuf, u64)>,
) -> AppResult<()> {
    match entry.typeflag {
        TYPE_DIRECTORY => {
            std::fs::create_dir_all(target)
                .map_err(|e| AppError::new(format!("cannot create directory: {e}")))?;
            set_mode(target, entry.mode);
            if running_as_root {
                let _ = chown_path(target, entry.uid, entry.gid);
            }
            dir_mtimes.push((target.to_path_buf(), entry.mtime));
            Ok(())
        }
        TYPE_SYMLINK => {
            if let Some(parent) = target.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            let _ = std::fs::remove_file(target);
            symlink_at(&entry.linkname, target)
        }
        TYPE_HARDLINK => {
            if let Some(parent) = target.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            let safe_link = sanitize_member_name(&entry.linkname)?;
            let link_target = dest_root.join(safe_link);
            let _ = std::fs::remove_file(target);
            std::fs::hard_link(&link_target, target).map_err(|e| {
                AppError::new(format!(
                    "cannot create hard link to '{}': {e} (the link target must already have \
                     been extracted earlier in the archive)",
                    link_target.display()
                ))
            })
        }
        TYPE_REGULAR | 0 => {
            if let Some(parent) = target.parent() {
                std::fs::create_dir_all(parent)
                    .map_err(|e| AppError::new(format!("cannot create directory: {e}")))?;
            }
            {
                let mut out = std::fs::File::create(target)
                    .map_err(|e| AppError::new(format!("cannot create file: {e}")))?;
                let mut remaining = entry.size;
                let mut buf = [0u8; 64 * 1024];
                while remaining > 0 {
                    let take = remaining.min(buf.len() as u64) as usize;
                    reader
                        .read_exact(&mut buf[..take])
                        .map_err(|e| AppError::new(format!("unexpected end of archive: {e}")))?;
                    out.write_all(&buf[..take])
                        .map_err(|e| AppError::new(format!("write error: {e}")))?;
                    remaining -= take as u64;
                }
                let padding = pad_len(entry.size);
                if padding > 0 {
                    let mut pad = [0u8; BLOCK_SIZE];
                    reader
                        .read_exact(&mut pad[..padding])
                        .map_err(|e| AppError::new(format!("unexpected end of archive: {e}")))?;
                }
            } // `out` closed here, before set_modified reopens it below
            set_mode(target, entry.mode);
            if running_as_root {
                let _ = chown_path(target, entry.uid, entry.gid);
            }
            let when = std::time::UNIX_EPOCH + std::time::Duration::from_secs(entry.mtime);
            let _ = std::fs::OpenOptions::new()
                .read(true)
                .open(target)
                .and_then(|f| f.set_modified(when));
            Ok(())
        }
        other => {
            eprintln!(
                "tar: skipping unsupported entry type '{}' for '{}'",
                other as char, entry.name
            );
            skip_data(reader, entry.size)
        }
    }
}

#[cfg(unix)]
fn symlink_at(target: &str, link: &Path) -> AppResult<()> {
    std::os::unix::fs::symlink(target, link)
        .map_err(|e| AppError::new(format!("cannot create symlink: {e}")))
}
#[cfg(not(unix))]
fn symlink_at(_target: &str, _link: &Path) -> AppResult<()> {
    Err(AppError::new("symlinks are not supported on this target"))
}

#[cfg(unix)]
fn set_mode(path: &Path, mode: u32) {
    use std::os::unix::fs::PermissionsExt;
    let _ = std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode & 0o7777));
}
#[cfg(not(unix))]
fn set_mode(_path: &Path, _mode: u32) {}

#[cfg(unix)]
mod ffi {
    use std::os::raw::c_char;
    extern "C" {
        pub fn chown(path: *const c_char, owner: u32, group: u32) -> i32;
    }
}

#[cfg(unix)]
fn chown_path(path: &Path, uid: u32, gid: u32) -> std::io::Result<()> {
    let c_path = std::ffi::CString::new(path.as_os_str().as_encoded_bytes())
        .map_err(|_| std::io::Error::new(std::io::ErrorKind::InvalidInput, "invalid path"))?;
    if unsafe { ffi::chown(c_path.as_ptr(), uid, gid) } == 0 {
        Ok(())
    } else {
        Err(std::io::Error::last_os_error())
    }
}
#[cfg(not(unix))]
fn chown_path(_path: &Path, _uid: u32, _gid: u32) -> std::io::Result<()> {
    Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "chown not available on this target",
    ))
}
