# fish completion for flighttape
complete -c flighttape -n '__fish_use_subcommand' -a 'convert' -d 'flighttape convert'
complete -c flighttape -n '__fish_use_subcommand' -a 'convert-dir' -d 'flighttape convert-dir'
complete -c flighttape -n '__fish_use_subcommand' -a 'validate' -d 'flighttape validate'
complete -c flighttape -n '__fish_use_subcommand' -a 'digest' -d 'flighttape digest'
complete -c flighttape -n '__fish_use_subcommand' -a 'verify' -d 'flighttape verify'
complete -c flighttape -n '__fish_use_subcommand' -a 'play' -d 'flighttape play'
complete -c flighttape -n '__fish_use_subcommand' -a 'compare' -d 'flighttape compare'
complete -c flighttape -n '__fish_use_subcommand' -a 'rollup' -d 'flighttape rollup'
complete -c flighttape -n '__fish_use_subcommand' -a 'slice' -d 'flighttape slice'
complete -c flighttape -n '__fish_use_subcommand' -a 'screen' -d 'flighttape screen'
complete -c flighttape -s h -l help -d 'Show help'
complete -c flighttape -n '__fish_seen_subcommand_from convert' -s 'o' -r -F
complete -c flighttape -n '__fish_seen_subcommand_from convert' -l 'output' -r -F
complete -c flighttape -n '__fish_seen_subcommand_from convert-dir' -s 'o' -r -F
complete -c flighttape -n '__fish_seen_subcommand_from convert-dir' -l 'output' -r -F
complete -c flighttape -n '__fish_seen_subcommand_from play' -l 'from' -r
complete -c flighttape -n '__fish_seen_subcommand_from play' -l 'to' -r
complete -c flighttape -n '__fish_seen_subcommand_from slice' -l 'from' -r
complete -c flighttape -n '__fish_seen_subcommand_from slice' -l 'to' -r
complete -c flighttape -n '__fish_seen_subcommand_from slice' -s 'o' -r -F
complete -c flighttape -n '__fish_seen_subcommand_from slice' -l 'output' -r -F
complete -c flighttape -n '__fish_seen_subcommand_from screen' -s 'o' -r -F
complete -c flighttape -n '__fish_seen_subcommand_from screen' -l 'output' -r -F
