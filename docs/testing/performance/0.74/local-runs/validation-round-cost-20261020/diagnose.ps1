$ErrorActionPreference='Stop'
function Store([string]$Name,$Value){if(Test-Path "$PSScriptRoot/$Name.json"){throw 'Never replace an observation'};$Value|ConvertTo-Json -Depth 12|Set-Content -Encoding utf8NoBOM "$PSScriptRoot/$Name.json"}
function Native([string]$Name,[string[]]$Arguments){
    if(Test-Path "$PSScriptRoot/$Name.log"){throw 'Never replace an observation'}
    & wsl.exe @Arguments *> "$PSScriptRoot/$Name.log"
    $taskExit=[int]$LASTEXITCODE
    Store "$Name-exit" ([ordered]@{source=(& git rev-parse HEAD).Trim();exit=$taskExit;recipe_sha256=(Get-FileHash $PSCommandPath -Algorithm SHA256).Hash.ToLowerInvariant();read_only=$true;server=$false})
}
Native 'wsl-state-before' @('--list','--verbose')
Native 'wsl-version' @('--version')
$taskConfig=Join-Path $env:USERPROFILE '.wslconfig'
Store 'windows-wsl-config' ([ordered]@{exists=(Test-Path -LiteralPath $taskConfig);sha256=$(if(Test-Path -LiteralPath $taskConfig){(Get-FileHash -LiteralPath $taskConfig -Algorithm SHA256).Hash.ToLowerInvariant()});selected_lines=@(if(Test-Path -LiteralPath $taskConfig){Get-Content -LiteralPath $taskConfig|Where-Object{$_ -match '^\s*(\[|vmIdleTimeout|memory|processors|kernelCommandLine|autoMemoryReclaim|nestedVirtualization|guiApplications|debugConsole|networkingMode)'}})})
try {
    $taskEvents=@(Get-WinEvent -FilterHashtable @{LogName='System';ProviderName='Microsoft-Windows-Kernel-General';Id=1;StartTime=(Get-Date).AddDays(-2)} -MaxEvents 40 -ErrorAction Stop|ForEach-Object{[ordered]@{record=$_.RecordId;time_created=$_.TimeCreated.ToUniversalTime().ToString('o');message=$_.Message}})
    Store 'windows-time-events' ([ordered]@{available=$true;events=$taskEvents})
}catch{Store 'windows-time-events' ([ordered]@{available=$false;error=$_.Exception.Message})}
Native 'linux-before' @('-d','Ubuntu','-u','root','--','sh','-c','printf "boot "; cat /proc/sys/kernel/random/boot_id; uname -a; printf "wsl.conf\n"; if test -f /etc/wsl.conf; then cat /etc/wsl.conf; fi; printf "clocksource\n"; cat /sys/devices/system/clocksource/clocksource0/current_clocksource /sys/devices/system/clocksource/clocksource0/available_clocksource; printf "journal-boots\n"; journalctl --list-boots --no-pager; printf "timesync-properties\n"; systemctl show systemd-timesyncd.service -p ActiveState -p SubState -p ActiveEnterTimestampMonotonic -p ExecMainStartTimestampMonotonic; printf "timesync-log\n"; journalctl -u systemd-timesyncd.service -b --no-pager -o short-monotonic -n 80; printf "kernel-time-log\n"; journalctl -k -b --no-pager -o short-monotonic -n 300 | grep -Ei "clock|time|tsc|hyper-v|suspend|resume"; printf "journal-storage\n"; ls -ld /var/log/journal /run/log/journal 2>/dev/null; true')
$taskInfo=[Diagnostics.ProcessStartInfo]::new('wsl.exe')
foreach($taskArg in @('-d','Ubuntu','--','sh','-c','read -r boot < /proc/sys/kernel/random/boot_id; exec python3 /mnt/c/Workspace/prj/jq/cashe/hydracache-074/docs/testing/performance/0.74/local-runs/validation-round-cost-20261020/clock_probe.py "$boot"')){[void]$taskInfo.ArgumentList.Add($taskArg)}
$taskInfo.UseShellExecute=$false;$taskInfo.RedirectStandardOutput=$true;$taskInfo.RedirectStandardError=$true
$taskProcess=[Diagnostics.Process]::new();$taskProcess.StartInfo=$taskInfo
if(Test-Path "$PSScriptRoot/clock-observations.json"){throw 'Never replace clock observations'}
[void]$taskProcess.Start();$taskStderr=$taskProcess.StandardError.ReadToEndAsync();$taskSamples=[Collections.Generic.List[object]]::new()
while($null -ne ($taskLine=$taskProcess.StandardOutput.ReadLine())){
    $taskSamples.Add([ordered]@{windows_utc_ticks=[DateTime]::UtcNow.Ticks;windows_qpc=[Diagnostics.Stopwatch]::GetTimestamp();linux=($taskLine|ConvertFrom-Json)})
}
$taskProcess.WaitForExit();$taskExit=$taskProcess.ExitCode
Store 'clock-observations' ([ordered]@{source=(& git rev-parse HEAD).Trim();exit=$taskExit;stderr=$taskStderr.GetAwaiter().GetResult();qpc_frequency=[Diagnostics.Stopwatch]::Frequency;recipe_sha256=(Get-FileHash $PSCommandPath -Algorithm SHA256).Hash.ToLowerInvariant();probe_sha256=(Get-FileHash "$PSScriptRoot/clock_probe.py" -Algorithm SHA256).Hash.ToLowerInvariant();read_only=$true;changes_time=$false;changes_service=$false;samples=$taskSamples;server=$false})
Native 'linux-after' @('-d','Ubuntu','--','sh','-c','printf "boot "; cat /proc/sys/kernel/random/boot_id; printf "timesync-log\n"; journalctl -u systemd-timesyncd.service -b --no-pager -o short-monotonic -n 30; printf "uptime\n"; cat /proc/uptime')
Native 'wsl-state-after' @('--list','--verbose')
if($taskExit -ne 0 -or $taskSamples.Count -ne 180){throw 'Read only observation incomplete'}
