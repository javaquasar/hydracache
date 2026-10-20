$ErrorActionPreference='Stop'
if((Test-Path "$PSScriptRoot/local-postflight.log") -or (Test-Path "$PSScriptRoot/local-postflight-exit.json")){throw 'Never replace postflight'}
& wsl.exe -d Ubuntu -u root -- sh -c 'cat /proc/sys/kernel/random/boot_id; find /tmp /sys/fs/cgroup -maxdepth 1 -name "hc074-root-fixture-*" -print' *> "$PSScriptRoot/local-postflight.log"
$taskExit=[int]$LASTEXITCODE
[ordered]@{source=(& git rev-parse HEAD).Trim();exit=$taskExit;recipe_sha256=(Get-FileHash $PSCommandPath -Algorithm SHA256).Hash.ToLowerInvariant();server=$false;product=$false}|ConvertTo-Json|Set-Content -Encoding utf8NoBOM "$PSScriptRoot/local-postflight-exit.json"
if($taskExit -ne 0){throw 'Postflight failed'}
