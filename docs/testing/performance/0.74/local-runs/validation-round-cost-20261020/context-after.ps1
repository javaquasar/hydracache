$ErrorActionPreference='Stop'
if(Test-Path "$PSScriptRoot/context-after.log"){throw 'Never replace context'}
& wsl.exe -d Ubuntu -- sh -c 'printf "boot "; cat /proc/sys/kernel/random/boot_id; printf "wsl.conf\n"; cat /etc/wsl.conf; printf "clocksource\n"; cat /sys/devices/system/clocksource/clocksource0/current_clocksource; printf "uptime\n"; cat /proc/uptime' *> "$PSScriptRoot/context-after.log"
$taskExit=[int]$LASTEXITCODE
$taskConfig=Join-Path $env:USERPROFILE '.wslconfig'
[ordered]@{source=(& git rev-parse HEAD).Trim();exit=$taskExit;recipe_sha256=(Get-FileHash $PSCommandPath -Algorithm SHA256).Hash.ToLowerInvariant();windows_wsl_config_exists=(Test-Path -LiteralPath $taskConfig);windows_wsl_config_sha256=$(if(Test-Path -LiteralPath $taskConfig){(Get-FileHash $taskConfig -Algorithm SHA256).Hash.ToLowerInvariant()});read_only=$true;server=$false;product=$false}|ConvertTo-Json|Set-Content -Encoding utf8NoBOM "$PSScriptRoot/context-after-exit.json"
if($taskExit -ne 0){throw 'Context observation failed'}
