param([Parameter(Mandatory)][ValidatePattern('^[0-9a-f]{40}$')][string]$Source)
$ErrorActionPreference='Stop'
if((Test-Path (Join-Path $PSScriptRoot 'inventory.json')) -or (Test-Path (Join-Path $PSScriptRoot 'held-session'))){throw 'Retain existing cohort'}
$taskStart=[Diagnostics.ProcessStartInfo]::new('wsl.exe')
$taskStart.UseShellExecute=$false;$taskStart.CreateNoWindow=$true;$taskStart.WindowStyle='Hidden'
$taskStart.RedirectStandardInput=$true;$taskStart.RedirectStandardOutput=$true;$taskStart.RedirectStandardError=$true
foreach($taskArg in @('-d','Ubuntu','--','python3','/mnt/c/Workspace/prj/jq/cashe/hydracache-074/docs/testing/performance/0.74/local-runs/held-consuming-join-20261020/clock-session.py')){$taskStart.ArgumentList.Add($taskArg)}
$taskProcess=[Diagnostics.Process]::Start($taskStart)
$taskLines=[Collections.Generic.List[string]]::new();$taskTail=$null;$taskComplete=$false
try{
    do{$taskLine=$taskProcess.StandardOutput.ReadLine();if($null -eq $taskLine){throw 'Clock observer ended before ready'};$taskLines.Add($taskLine);Write-Host $taskLine;$taskEvent=$taskLine|ConvertFrom-Json}while($taskEvent.event -cne 'ready')
    $taskProcess.StandardInput.WriteLine('begin');$taskProcess.StandardInput.Flush()
    do{$taskLine=$taskProcess.StandardOutput.ReadLine();if($null -eq $taskLine){throw 'Clock observer ended before begin'};$taskLines.Add($taskLine);Write-Host $taskLine;$taskEvent=$taskLine|ConvertFrom-Json}while($taskEvent.event -cne 'begin')
    $taskTail=$taskProcess.StandardOutput.ReadToEndAsync()
    foreach($taskMode in @('194','188')){& (Join-Path $PSScriptRoot 'checks.ps1') -Mode $taskMode -Source $Source;if(-not $?){throw "Cohort refused: $taskMode"}}
    $taskComplete=$true
}finally{
    if(-not $taskProcess.HasExited){$taskProcess.StandardInput.WriteLine('finish');$taskProcess.StandardInput.Flush();$taskProcess.StandardInput.Close()}
    if($null -eq $taskTail){$taskTail=$taskProcess.StandardOutput.ReadToEndAsync()}
    if(-not $taskProcess.WaitForExit(10000)){$taskProcess.Kill();throw 'Owned observer did not terminate'}
    foreach($taskLine in ($taskTail.GetAwaiter().GetResult() -split "`r?`n")){if($taskLine){$taskLines.Add($taskLine)}}
    $taskLines | Set-Content (Join-Path $PSScriptRoot 'held-clock-session.log') -Encoding utf8NoBOM
    $taskEvents=@($taskLines|ForEach-Object{$_|ConvertFrom-Json})
    $taskBackwards=@($taskEvents|Where-Object {$_.event -ceq 'backwards' -and $_.phase -ceq 'tests'})
    [ordered]@{source=$Source;complete=$taskComplete;observer_exit=$taskProcess.ExitCode;test_phase_backwards=$taskBackwards.Count;single_wsl_process=$true;service_changed=$false;manual_clock_changed=$false;windows_clock_changed=$false;server=$false;supervisor=$false;recipe=(Get-FileHash $PSCommandPath -Algorithm SHA256).Hash.ToLowerInvariant();observer_recipe=(Get-FileHash (Join-Path $PSScriptRoot 'clock-session.py') -Algorithm SHA256).Hash.ToLowerInvariant()}|ConvertTo-Json|Set-Content (Join-Path $PSScriptRoot 'held-clock-session-receipt.json') -Encoding utf8NoBOM
    if($taskBackwards.Count -ne 0 -or $taskProcess.ExitCode -ne 0){throw 'Clock observer refused the test phase'}
    $taskProcess.Dispose()
}
foreach($taskMode in @('windows','repository')){& (Join-Path $PSScriptRoot 'checks.ps1') -Mode $taskMode -Source $Source;if(-not $?){throw "Cohort refused: $taskMode"}}
