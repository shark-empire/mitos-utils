# Command reference

One-line usage for every utility in `mitos-utils`. See
docs/compatibility.md for exactly which flags each one supports.

Every command below also accepts `--help` (prints its usage and
exits) and `--version`, and every command that takes file/path
arguments accepts `--` to mark the end of options (so a file
literally named `-oddfile` can still be referenced). Every one is
also reachable through the single multiplexed binary,
`mitos-box <name> [args...]` -- see `mitos-box` below.

## Filesystem

- `cat [-n|-b] [FILE...]` -- concatenate files to stdout.
- `ls [-a] [-l] [-h] [PATH...]` -- list directory contents.
- `mkdir [-p] DIR...` -- create directories.
- `rmdir DIR...` -- remove empty directories.
- `touch FILE...` -- create files / update modification time.
- `cp [-r] [-i] [-p] SOURCE... DEST` -- copy files or directory trees (`-i` confirms overwrites, `-p` preserves modification time).
- `mv [-i] SOURCE... DEST` -- move/rename files or directories (`-i` confirms overwrites).
- `rm [-r] [-f] [-i] FILE...` -- remove files or directory trees (`-i` confirms each removal).
- `ln [-s] [-f] TARGET LINK_NAME` -- create a hard or symbolic link.
- `pwd` -- print the current working directory.
- `basename PATH [SUFFIX]` -- strip directory (and suffix) from a path.
- `dirname PATH...` -- strip the last component from a path.
- `realpath PATH...` -- resolve to an absolute, symlink-free path.
- `readlink [-f] PATH...` -- print a symlink's target, or its fully resolved path with `-f`.
- `stat PATH...` -- print detailed file status.

## Text processing

- `echo [-n] [-e] TEXT...` -- print arguments.
- `printf FORMAT [ARG...]` -- formatted output.
- `head [-n N] [-c N] [FILE...]` -- print the first N lines (default 10) or bytes.
- `tail [-n N] [-c N] [FILE...]` -- print the last N lines (default 10) or bytes.
- `grep [-i] [-v] [-n] PATTERN [FILE...]` -- print matching lines.
- `sort [-r] [-n] [-u] [-k N] [-t DELIM] [FILE...]` -- sort lines (optionally by field N).
- `uniq [-c] [-d] [FILE]` -- collapse adjacent duplicate lines.
- `wc [-l] [-w] [-c] [FILE...]` -- count lines/words/bytes.
- `cut -d DELIM -f LIST [FILE...]` -- extract fields from each line.
- `tr SET1 SET2` / `tr -d SET1` -- translate or delete characters on stdin.
- `tee [-a] FILE...` -- copy stdin to stdout and to files.
- `diff FILE1 FILE2` -- report line-by-line differences.

## Process / system

- `ps` -- list running processes (Linux).
- `kill [-SIGNAL] PID...` -- send a signal to a process.
- `sleep DURATION` -- pause (accepts `s`/`m`/`h` suffixes).
- `uptime` -- print system uptime (Linux).
- `free` -- print memory usage (Linux).
- `uname [-a|-s|-n|-r|-m]` -- print system information.
- `hostname` -- print the system hostname.
- `env` -- print the environment.
- `printenv [NAME...]` -- print named environment variables (or all).
- `whoami` -- print the current username.
- `id` -- print uid/gid/groups for the current user.
- `groups` -- print the current user's groups.

## Disk / mount

- `df [PATH...]` -- report filesystem space usage (Linux).
- `du [-h] [-s] [PATH...]` -- estimate directory space usage.
- `mount` / `mount SOURCE TARGET -t FSTYPE` -- list or create mounts.
- `umount TARGET` -- unmount a filesystem.
- `sync` -- flush filesystem buffers to disk.
- `dmesg` -- print the kernel ring buffer (Linux).

## Permissions

- `chmod [-R] MODE FILE...` -- change file mode (octal or symbolic).
- `chown [-R] OWNER[:GROUP] FILE...` -- change file owner/group.
- `chgrp [-R] GROUP FILE...` -- change file group.

## Misc

- `clear` -- clear the terminal screen.
- `true` -- exit successfully, doing nothing.

## Multiplexed binary

- `mitos-box <name> [args...]` -- act as any applet above, by name.
  Also usable via a symlink/hardlink named after the applet (e.g.
  `ln -s mitos-box cat`) -- see docs/architecture.md.
