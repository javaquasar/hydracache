$ErrorActionPreference='Stop'
$lines=Get-Content (Join-Path $PSScriptRoot 'host.log')
$compiler=$null
$rows=@();$points=@();$rounds=@()
$stats='Stats \{ wall_ns: ([0-9]+), authentication_calls: ([0-9]+), authentication_ns: ([0-9]+), signature_calls: ([0-9]+), signature_ns: ([0-9]+), worker_policy_calls: ([0-9]+), worker_policy_ns: ([0-9]+) \}'
function Values($match,[int]$offset){
    @{wall_ns=[uint64]$match.Groups[$offset].Value;authentication_calls=[uint64]$match.Groups[$offset+1].Value;authentication_ns=[uint64]$match.Groups[$offset+2].Value;signature_calls=[uint64]$match.Groups[$offset+3].Value;signature_ns=[uint64]$match.Groups[$offset+4].Value;worker_policy_calls=[uint64]$match.Groups[$offset+5].Value;worker_policy_ns=[uint64]$match.Groups[$offset+6].Value}
}
foreach($line in $lines){
    $m=[regex]::Match($line,'HC074-PREPARATION-HOST compiler=(194|188) begin=true start=false')
    if($m.Success){$compiler=$m.Groups[1].Value}
    $m=[regex]::Match($line,"HC074-PREPARATION-COST sample=([0-2]) bucket=([a-z-]+) stats=$stats")
    if($m.Success){if(!$compiler){throw 'unowned row'};$row=Values $m 3;$row.compiler=$compiler;$row.sample=[int]$m.Groups[1].Value;$row.bucket=$m.Groups[2].Value;$rows+=@($row)}
    $m=[regex]::Match($line,"HC074-PREPARATION-ROUND sample=([0-2]) pass=([01]) channel_checks=2 stats=$stats")
    if($m.Success){$row=Values $m 3;$row.compiler=$compiler;$row.sample=[int]$m.Groups[1].Value;$row.pass=[int]$m.Groups[2].Value;$rounds+=@($row)}
    $m=[regex]::Match($line,'HC074-PREPARATION-POINT sample=([0-2]) point=([a-z-]+) issued_ns=([0-9]+) now_ns=([0-9]+) age_ns=([0-9]+)')
    if($m.Success){$points+=@(@{compiler=$compiler;sample=[int]$m.Groups[1].Value;point=$m.Groups[2].Value;issued_ns=[uint64]$m.Groups[3].Value;now_ns=[uint64]$m.Groups[4].Value;age_ns=[uint64]$m.Groups[5].Value})}
}
if($rows.Count -ne 48 -or $rounds.Count -ne 12 -or $points.Count -ne 48){throw 'incomplete diagnostic rows'}
foreach($compiler in @('194','188')){foreach($sample in 0..2){
    $owned=@($rows | Where-Object {$_.compiler -ceq $compiler -and $_.sample -eq $sample})
    if($owned.Count -ne 8 -or @($owned.bucket | Sort-Object -Unique).Count -ne 8){throw 'duplicate or missing bucket'}
    $ownedPoints=@($points | Where-Object {$_.compiler -ceq $compiler -and $_.sample -eq $sample})
    if($ownedPoints.Count -ne 8 -or @($ownedPoints.issued_ns | Sort-Object -Unique).Count -ne 1){throw 'original origin differs'}
    $last=0
    foreach($point in $ownedPoints){if($point.now_ns -lt $last -or $point.now_ns - $point.issued_ns -ne $point.age_ns -or $point.age_ns -ge 10000000000){throw 'original freshness'};$last=$point.now_ns}
}}
@{kind='correctness-attribution';product_performance_claim=$false;observation_schedule_changed=$false;rows=$rows;rounds=$rounds;points=$points} | ConvertTo-Json -Depth 8 | Set-Content -Encoding utf8NoBOM (Join-Path $PSScriptRoot 'observations.json')
'HC074-PREPARATION-ANALYSIS rows=48 rounds=12 points=48 admission=false'
