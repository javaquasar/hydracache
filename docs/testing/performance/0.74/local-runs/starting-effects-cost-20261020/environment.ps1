$ErrorActionPreference='Stop'
foreach($taskProbe in @(@('kernel',@('uname','-a')),@('tmp-filesystem',@('findmnt','--target','/tmp','--output','TARGET,SOURCE,FSTYPE,OPTIONS','--json')),@('local-boot',@('cat','/proc/sys/kernel/random/boot_id')))){
    $taskName=$taskProbe[0];$taskArgs=$taskProbe[1]
    if((Test-Path "$PSScriptRoot/$taskName.log") -or (Test-Path "$PSScriptRoot/$taskName-exit.json")){throw 'Never replace an observation'}
    & wsl.exe -d Ubuntu -- @taskArgs *> "$PSScriptRoot/$taskName.log"
    $taskExit=[int]$LASTEXITCODE
    [ordered]@{source=(& git rev-parse HEAD).Trim();selection=$taskName;exit=$taskExit;recipe_sha256=(Get-FileHash $PSCommandPath -Algorithm SHA256).Hash.ToLowerInvariant();server=$false;product=$false}|ConvertTo-Json|Set-Content -Encoding utf8NoBOM "$PSScriptRoot/$taskName-exit.json"
    if($taskExit -ne 0){throw 'Environment observation failed'}
}
