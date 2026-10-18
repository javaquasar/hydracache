param([string]$Directory)
$ErrorActionPreference='Stop'
if (!$Directory) {$Directory=Join-Path (Resolve-Path (Join-Path $PSScriptRoot '../../../../../..')).Path 'target/boot-buffer-plain-20261018'}
function Load([string]$Name) {return ,@(Get-Content -LiteralPath (Join-Path $Directory "$Name-inputs.json") -Raw | ConvertFrom-Json -AsHashtable)}
$a=Load 'Baseline-194';$b=Load 'Baseline-188';$c=Load 'Candidate-194';$d=Load 'Candidate-188'
foreach ($set in @($a,$b,$c,$d)) {
    if ($set.Count -ne 367 -or @($set.path | Select-Object -Unique).Count -ne 367) {throw 'input inventory'}
}
for ($i=0; $i -lt 367; $i++) {
    foreach ($set in @($b,$c,$d)) {if ($set[$i].path -cne $a[$i].path) {throw 'path order'}}
    foreach ($field in @('raw_sha256','lf_sha256')) {
        if ($a[$i][$field] -cne $b[$i][$field] -or $c[$i][$field] -cne $d[$i][$field]) {throw 'within-role compiler input drift'}
    }
}
$delta=@(for ($i=0; $i -lt 367; $i++) {if ($a[$i].raw_sha256 -cne $c[$i].raw_sha256 -or $a[$i].lf_sha256 -cne $c[$i].lf_sha256) {$a[$i].path}})
if ($delta.Count -ne 1 -or $delta[0] -cne 'tools/long-run-supervisor-074/src/diagnostic_kernel_origin.rs') {throw 'comparison is not one isolated input'}
foreach ($role in @('Baseline','Candidate')) {
    if ([IO.File]::ReadAllText((Join-Path $Directory "$role-194-origin.source")) -cne [IO.File]::ReadAllText((Join-Path $Directory "$role-188-origin.source"))) {throw 'origin capsule drift'}
}
'HC074-PLAIN-INPUTS total=367 differing=1 origin_only=true within_role_identical=true'
