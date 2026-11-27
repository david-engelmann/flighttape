# bash completion for flighttape
_flighttape() {
    local cur prev words cword
    cur="${COMP_WORDS[COMP_CWORD]}"
    prev="${COMP_WORDS[COMP_CWORD-1]}"
    if [[ ${COMP_CWORD} -eq 1 ]]; then
        COMPREPLY=( $(compgen -W "convert convert-dir validate digest verify play compare rollup slice screen -h --help" -- "${cur}") )
        return 0
    fi
    case "${cur}" in
        -*)
            COMPREPLY=( $(compgen -W "-o --output --from --to -h --help -V --version" -- "${cur}") )
            return 0
            ;;
    esac
    COMPREPLY=( $(compgen -f -- "${cur}") )
}
complete -F _flighttape flighttape
