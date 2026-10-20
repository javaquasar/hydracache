$ErrorActionPreference='Stop'
$taskInfo=[Diagnostics.ProcessStartInfo]::new('wsl.exe')
foreach($taskArg in @('-d','Ubuntu','--','python3','/mnt/c/Workspace/prj/jq/cashe/hydracache-074/docs/testing/performance/0.74/local-runs/validation-round-cost-20261020/clock_probe_v2.py')){[void]$taskInfo.ArgumentList.Add($taskArg)}
$taskInfo.UseShellExecute=$false;$taskInfo.RedirectStandardOutput=$true;$taskInfo.RedirectStandardError=$true
$taskProcess=[Diagnostics.Process]::new();$taskProcess.StartInfo=$taskInfo
if(Test-Path "$PSScriptRoot/clock-observations-v2.json"){throw 'Never replace observations'}
[void]$taskProcess.Start();$taskStderr=$taskProcess.StandardError.ReadToEndAsync();$taskSamples=[Collections.Generic.List[object]]::new()
while($null -ne ($taskLine=$taskProcess.StandardOutput.ReadLine())){
    $taskSamples.Add([ordered]@{windows_utc_ticks=[DateTime]::UtcNow.Ticks;windows_qpc=[Diagnostics.Stopwatch]::GetTimestamp();linux=($taskLine|ConvertFrom-Json)})
}
$taskProcess.WaitForExit();$taskExit=$taskProcess.ExitCode
[ordered]@{source=(& git rev-parse HEAD).Trim();exit=$taskExit;stderr=$taskStderr.GetAwaiter().GetResult();qpc_frequency=[Diagnostics.Stopwatch]::Frequency;recipe_sha256=(Get-FileHash $PSCommandPath -Algorithm SHA256).Hash.ToLowerInvariant();probe_sha256=(Get-FileHash "$PSScriptRoot/clock_probe_v2.py" -Algorithm SHA256).Hash.ToLowerInvariant();read_only=$true;changes_time=$false;changes_service=$false;samples=$taskSamples;server=$false}|ConvertTo-Json -Depth 10|Set-Content -Encoding utf8NoBOM "$PSScriptRoot/clock-observations-v2.json"
if($taskExit -ne 0 -or $taskSamples.Count -ne 180){throw 'Read only observation incomplete'}
