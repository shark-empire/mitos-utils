# Command reference

One-line usage for every utility in `mitos-utils`. See
docs/compatibility.md for exactly which flags each one supports.

## Filesystem

- `cat [-n|-b] [FILE...]` -- concatenate files to stdout.
- `ls [-a] [-l] [-h] [PATH...]` -- list directory contents.
- `mkdir [-p] DIR...` -- create directories.
- `rmdir DIR...` -- remove empty directories.
- `touch FILE...` -- create files / update modification time.
- `cp [-r] SOURCE... DEST` -- copy files or directory trees.
- `mv SOURCE... DEST` -- move/rename files or directories.
- `rm [-r] [-f] FILE...` -- remove files or directory trees.
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
- `head [-n N] [FILE...]` -- print the first N lines (default 10).
- `tail [-n N] [FILE...]` -- print the last N lines (default 10).
- `grep [-i] [-v] [-n] PATTERN [FILE...]` -- print matching lines.
- `sort [-r] [-n] [-u] [FILE...]` -- sort lines.
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
