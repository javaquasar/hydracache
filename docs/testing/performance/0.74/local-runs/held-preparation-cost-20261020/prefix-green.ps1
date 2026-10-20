param([Parameter(Mandatory)][ValidatePattern('^[0-9a-f]{40}$')][string]$Source)
$ErrorActionPreference='Stop'
if((& git rev-parse HEAD).Trim() -cne $Source){throw 'Wrong candidate source'}
foreach($taskLane in '194','188'){
    if(Test-Path "$PSScriptRoot/prefix-green-$taskLane.log"){throw 'Do not replay a recorded sample'}
    $taskVersion=if($taskLane -ceq '194'){'1.94.0'}else{'1.88.0'}
    & wsl.exe -d Ubuntu -- env "CARGO_TARGET_DIR=/home/artur/.cache/hydracache-074-$taskLane-check" /home/artur/.cargo/bin/cargo "+$taskVersion" test --locked --release -p hydracache-long-run-supervisor-074 --lib --features fixed-leaf-alloc-probe projection_prefix -- --nocapture --test-threads=1 2>&1|Tee-Object "$PSScriptRoot/prefix-green-$taskLane.log"|Out-Null
    $taskExit=$LASTEXITCODE
    [ordered]@{source=$Source;lane=$taskLane;exit=$taskExit;feature='fixed-leaf-alloc-probe';server=$false;product=$false}|ConvertTo-Json|Set-Content -Encoding utf8NoBOM "$PSScriptRoot/prefix-green-$taskLane-exit.json"
    if($taskExit -ne 0){throw 'Leaf candidate not accepted'}
    $taskRaw=Get-Content -Raw "$PSScriptRoot/prefix-green-$taskLane.log"
    $taskMatches=[regex]::Matches($taskRaw,'\((/home/artur/[^\r\n)]+/hydracache_long_run_supervisor_074-[0-9a-f]+)\)')
    if($taskMatches.Count -ne 1){throw 'Image selection'}
    $taskImage=$taskMatches[0].Groups[1].Value
    $taskHash=((& wsl.exe -d Ubuntu -- sha256sum $taskImage).Trim().Substring(0,64))
    [ordered]@{source=$Source;lane=$taskLane;path=$taskImage;sha256=$taskHash;feature='fixed-leaf-alloc-probe';server=$false;product=$false}|ConvertTo-Json|Set-Content -Encoding utf8NoBOM "$PSScriptRoot/prefix-green-$taskLane-image.json"
    Write-Output "prefix candidate lane=$taskLane accepted=true"
}
if((& git rev-parse HEAD).Trim() -cne $Source -or (& git status --porcelain -- tools/long-run-supervisor-074)){throw 'Candidate changed'}
