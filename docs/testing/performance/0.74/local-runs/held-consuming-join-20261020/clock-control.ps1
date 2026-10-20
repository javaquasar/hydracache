param([Parameter(Mandatory)][ValidatePattern('^[0-9a-f]{40}$')][string]$Source)
$ErrorActionPreference='Stop'
$taskBefore=(& wsl.exe -d Ubuntu -- systemctl is-active systemd-timesyncd.service).Trim()
if($LASTEXITCODE -ne 0 -or $taskBefore -cne 'active'){throw 'Require initially active local timesyncd'}
$taskRoot=(Resolve-Path (Join-Path $PSScriptRoot '../../../../../..')).Path
$taskProbe='/mnt/c/Workspace/prj/jq/cashe/hydracache-074/docs/testing/performance/0.74/local-runs/held-consuming-join-20261020/clock-probe.py'
if((Test-Path (Join-Path $PSScriptRoot 'inventory.json')) -or (Test-Path (Join-Path $PSScriptRoot 'stable-clock'))){throw 'Retain existing cohort'}
$taskComplete=$false
Push-Location $taskRoot
try {
    & wsl.exe -d Ubuntu -u root -- systemctl stop systemd-timesyncd.service
    if($LASTEXITCODE -ne 0){throw 'Stop exact local timesyncd failed'}
    $taskStopped=(& wsl.exe -d Ubuntu -- systemctl is-active systemd-timesyncd.service).Trim()
    if($taskStopped -cne 'inactive'){throw 'Exact local timesyncd is not inactive'}
    & wsl.exe -d Ubuntu -- python3 $taskProbe 2>&1 | Tee-Object (Join-Path $PSScriptRoot 'clock-probe-before.log')
    if($LASTEXITCODE -ne 0){throw 'Local clock probe refused'}
    foreach($taskMode in @('194','188')){
        & (Join-Path $PSScriptRoot 'checks.ps1') -Mode $taskMode -Source $Source
        if(-not $?){throw "Cohort refused: $taskMode"}
    }
    & wsl.exe -d Ubuntu -- python3 $taskProbe 2>&1 | Tee-Object (Join-Path $PSScriptRoot 'clock-probe-after.log')
    if($LASTEXITCODE -ne 0){throw 'Closing local clock probe refused'}
    $taskComplete=$true
}finally{
    & wsl.exe -d Ubuntu -u root -- systemctl start systemd-timesyncd.service
    if($LASTEXITCODE -ne 0){throw 'Restore exact local timesyncd failed'}
    $taskAfter=(& wsl.exe -d Ubuntu -- systemctl is-active systemd-timesyncd.service).Trim()
    if($LASTEXITCODE -ne 0 -or $taskAfter -cne 'active'){throw 'Local timesyncd not restored'}
    [ordered]@{source=$Source;complete=$taskComplete;local_timesyncd_before=$taskBefore;local_timesyncd_during=$taskStopped;local_timesyncd_after=$taskAfter;service='systemd-timesyncd.service';distribution='Ubuntu';controlled_modes=@('194','188');control_recipe=(Get-FileHash $PSCommandPath -Algorithm SHA256).Hash.ToLowerInvariant();probe_recipe=(Get-FileHash (Join-Path $PSScriptRoot 'clock-probe.py') -Algorithm SHA256).Hash.ToLowerInvariant();manual_clock_changed=$false;windows_touched=$false;server=$false;supervisor=$false}|ConvertTo-Json|Set-Content (Join-Path $PSScriptRoot 'clock-control-receipt.json') -Encoding utf8NoBOM
    Pop-Location
}
foreach($taskMode in @('windows','repository')){
    & (Join-Path $PSScriptRoot 'checks.ps1') -Mode $taskMode -Source $Source
    if(-not $?){throw "Cohort refused: $taskMode"}
}
