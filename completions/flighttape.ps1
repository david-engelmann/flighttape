# PowerShell completion for flighttape
Register-ArgumentCompleter -Native -CommandName flighttape -ScriptBlock {
    param($wordToComplete, $commandAst, $cursorPosition)
    $elements = $commandAst.CommandElements
    $subMap = @{ 'convert' = @('-o', '--output', '-h', '--help'), 'convert-dir' = @('-o', '--output', '-h', '--help'), 'validate' = @('-h', '--help'), 'digest' = @('-h', '--help'), 'verify' = @('-h', '--help'), 'play' = @('--from', '--to', '-h', '--help'), 'compare' = @('-h', '--help'), 'rollup' = @('-h', '--help'), 'slice' = @('--from', '--to', '-o', '--output', '-h', '--help'), 'screen' = @('-o', '--output', '-h', '--help') }
    if ($elements.Count -le 1 -or ($elements.Count -eq 2 -and $wordToComplete.Length -gt 0 -and -not $subMap.ContainsKey($elements[1].Extent.Text))) {
        $candidates = @('convert', 'convert-dir', 'validate', 'digest', 'verify', 'play', 'compare', 'rollup', 'slice', 'screen', '-h', '--help')
    } else {
        $sub = $elements[1].Extent.Text
        if ($subMap.ContainsKey($sub)) {
            $candidates = $subMap[$sub]
        } else {
            $candidates = @('-h', '--help')
        }
    }
    $candidates | Where-Object { $_ -like "$wordToComplete*" } | ForEach-Object {
        [System.Management.Automation.CompletionResult]::new($_, $_, 'ParameterValue', $_)
    }
}
