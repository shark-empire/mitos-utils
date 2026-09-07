#compdef mitos-box

_mitos_box() {
    local -a applets
    applets=(cat ls mkdir rmdir touch cp mv rm ln pwd basename dirname realpath readlink stat echo printf head tail grep sort uniq wc cut tr tee diff ps kill sleep uptime free uname hostname env printenv whoami id groups df du mount umount sync dmesg chmod chown chgrp clear true)
    _arguments "1: :($applets)" "*:: :->args"
}
_mitos_box "$@"
