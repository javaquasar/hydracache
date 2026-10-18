$ErrorActionPreference='Stop'
. (Join-Path $PSScriptRoot 'rows.ps1')
$rows=Read-KernelRows (Get-Content (Join-Path $PSScriptRoot 'host.log') -Raw)
@{unit='nanoseconds';iterations_per_sample=32;rows=$rows;exclusive_cpu_claim=$false;critical_path_claim=$false;optimization_accepted=$false} | ConvertTo-Json -Depth 8 | Set-Content -Encoding utf8NoBOM (Join-Path $PSScriptRoot 'observations.json')
foreach ($compiler in @('194','188')) {
    foreach ($index in 0..9) {
        $selected=@($rows | Where-Object {$_.compiler -ceq $compiler -and $_.type -ceq 'NODE' -and $_.index -eq $index})
        $range=$selected | ForEach-Object {$_.inclusive_ns / 32000000.0} | Measure-Object -Minimum -Maximum
        "compiler=$compiler node=$index per_worker_ms=$($range.Minimum.ToString('F6',[Globalization.CultureInfo]::InvariantCulture))..$($range.Maximum.ToString('F6',[Globalization.CultureInfo]::InvariantCulture))"
    }
}
'HC074-KERNEL-ANALYZE rows=174 numeric_only=true admission=false'
