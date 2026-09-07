_mitos_box() {
    local cur prev opts applets
    COMPREPLY=()
    cur="${COMP_WORDS[COMP_CWORD]}"
    prev="${COMP_WORDS[COMP_CWORD-1]}"
    
    applets="cat ls mkdir rmdir touch cp mv rm ln pwd basename dirname realpath readlink stat echo printf head tail grep sort uniq wc cut tr tee diff ps kill sleep uptime free uname hostname env printenv whoami id groups df du mount umount sync dmesg chmod chown chgrp clear true"
    
    if [[ ${COMP_CWORD} -eq 1 ]]; then
        COMPREPLY=( $(compgen -W "${applets}" -- "${cur}") )
        return 0
    fi
}
complete -F _mitos_box mitos-box
