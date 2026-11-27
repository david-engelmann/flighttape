# bash completion for flighttape
_flighttape() {
    local cur prev sub sub_opts line
    COMPREPLY=()
    cur="${COMP_WORDS[COMP_CWORD]}"
    prev="${COMP_WORDS[COMP_CWORD-1]}"
    if [[ ${COMP_CWORD} -eq 1 ]]; then
        COMPREPLY=( $(compgen -W "convert convert-dir validate digest verify play compare rollup slice screen -h --help" -- "${cur}") )
        return 0
    fi
    sub="${COMP_WORDS[1]}"
    sub_opts="-h --help"
    case "${sub}" in
        convert) sub_opts="-o --output -h --help" ;;
        convert-dir) sub_opts="-o --output -h --help" ;;
        validate) sub_opts="-h --help" ;;
        digest) sub_opts="-h --help" ;;
        verify) sub_opts="-h --help" ;;
        play) sub_opts="--from --to -h --help" ;;
        compare) sub_opts="-h --help" ;;
        rollup) sub_opts="-h --help" ;;
        slice) sub_opts="--from --to -o --output -h --help" ;;
        screen) sub_opts="-o --output -h --help" ;;
    esac
    case "${cur}" in
        -*)
            COMPREPLY=( $(compgen -W "${sub_opts}" -- "${cur}") )
            return 0
            ;;
    esac
    compopt -o filenames 2>/dev/null || true
    while IFS= read -r line; do
        COMPREPLY+=( "${line}" )
    done < <(compgen -f -- "${cur}")
}
complete -F _flighttape flighttape
