# PowerShell completion for flighttape
Register-ArgumentCompleter -Native -CommandName flighttape -ScriptBlock {
    param($wordToComplete, $commandAst, $cursorPosition)
    $candidates = @('convert', 'convert-dir', 'validate', 'digest', 'verify', 'play', 'compare', 'rollup', 'slice', 'screen', '-o', '--output', '--from', '--to', '-h', '--help', '-V', '--version')
    $candidates | Where-Object { $_ -like "$wordToComplete*" } | ForEach-Object {
        [System.Management.Automation.CompletionResult]::new($_, $_, 'ParameterValue', $_)
    }
}
