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
- `ls [-a] [-l] [-h] [-R] [-t] [-S] [PATH...]` -- list directory contents.
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
- `find [PATH...] [-name PATTERN] [-type f|d|l] [-maxdepth N] [-mindepth N]` -- walk a directory tree.
- `which COMMAND...` -- print the full path of each command found on `$PATH`.

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
- `pgrep PATTERN` -- list PIDs whose command name matches PATTERN.
- `pkill [-SIGNAL] PATTERN` -- signal every process whose command name matches PATTERN.
- `nice [-n ADJUSTMENT] COMMAND [ARGS...]` -- run COMMAND at an adjusted priority.
- `nproc` -- print the number of available processing units.
- `lscpu` -- print basic CPU information.
- `sleep DURATION` -- pause (accepts `s`/`m`/`h` suffixes).
- `date [+FORMAT]` -- print the current date and time (UTC).
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
- `lsblk` -- list block devices and their partitions.
- `mount` / `mount SOURCE TARGET -t FSTYPE` -- list or create mounts.
- `umount TARGET` -- unmount a filesystem.
- `sync` -- flush filesystem buffers to disk.
- `dmesg` -- print the kernel ring buffer (Linux).

## Permissions

- `chmod [-R] MODE FILE...` -- change file mode (octal or symbolic).
- `chown [-R] OWNER[:GROUP] FILE...` -- change file owner/group.
- `chgrp [-R] GROUP FILE...` -- change file group.

## Networking

- `ping [-c COUNT] HOST` -- send ICMPv4 echo requests (IPv4 only, default count 4).

## Privilege escalation

**`su`/`sudo` have not been independently security-reviewed -- see
README.md's security section before installing either setuid
anywhere real.**

- `su [USER] [-c COMMAND]` -- switch to USER's identity (default root), given their password.
- `sudo COMMAND [ARGS...]` -- run COMMAND as root, given the caller's own password and `/etc/mitos-sudoers` membership.

## Service control

- `service {status|reload|ping|targets|apps|isolate TARGET|launch PATH [ARGS...]|logs [FILTER]}` -- talk to mitos-services' control socket.

## Misc

- `clear` -- clear the terminal screen.
- `true` -- exit successfully, doing nothing.
- `false` -- exit unsuccessfully, doing nothing.

## Multiplexed binary

- `mitos-box <name> [args...]` -- act as any applet above, by name.
  Also usable via a symlink/hardlink named after the applet (e.g.
  `ln -s mitos-box cat`) -- see docs/architecture.md.
