param([Parameter(Mandatory)][ValidatePattern('^[0-9a-f]{40}$')][string]$Source)
$ErrorActionPreference='Stop'
$taskRepo=(Resolve-Path (Join-Path $PSScriptRoot '../..')).Path
Set-Location $taskRepo
$taskPacket=Join-Path $taskRepo 'docs/testing/performance/0.74/local-runs/observer-topology-cost-20261021'
if((& git rev-parse HEAD).Trim() -cne $Source -or (& git status --porcelain --untracked-files=no)){throw 'Exact clean source required'}
& git diff --quiet b074f8f9f3d8f9766b3b04c59a8291941a7e3b1d HEAD -- tools/long-run-supervisor-074 crates Cargo.lock
if($LASTEXITCODE -ne 0){throw 'Live runtime/observers changed'}
New-Item -ItemType Directory -Path $taskPacket -ErrorAction Stop|Out-Null
function Hash([string]$Path){(Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant()}
$taskSteps=@();$taskImages=@{}
function Record([string]$Name,[scriptblock]$Call,[bool]$Native=$true){
    $taskLog=Join-Path $taskPacket "$Name.log"
    & $Call *> $taskLog
    $taskOk=$?;$taskCode=if($Native){[int]$LASTEXITCODE}else{if($taskOk){0}else{1}}
    $script:taskSteps+=@{name=$Name;exit=$taskCode;log=[IO.Path]::GetFileName($taskLog);sha256=(Hash $taskLog)}
    if($taskCode -ne 0){throw "Failed $Name"}
}
$taskFailure=$null
try{
    Record 'prior-review' { & ./docs/testing/performance/0.74/local-runs/partial-preparation-attribution-20261021/review.ps1 -Mode Review } $false
    Record 'topology' { & python -B tools/observer-topology-profile-074/topology.py --output (Join-Path $taskPacket 'input.json') }
    Record 'reader-tests' { & python -B tools/observer-topology-profile-074/test_review.py }
    foreach($taskLane in @('194','188')){
        $taskVersion=if($taskLane -ceq '194'){'1.94.0'}else{'1.88.0'}
        foreach($taskAction in @('test','check','clippy','build')){
            $taskArgs=@($taskAction,'--offline','--locked','--manifest-path','tools/observer-topology-profile-074/Cargo.toml')
            switch($taskAction){
                'test'{$taskArgs+=@('--','--test-threads=1')}
                'check'{$taskArgs+='--all-targets'}
                'clippy'{$taskArgs+=@('--all-targets','--all-features','--','-D','warnings')}
                'build'{$taskArgs+='--release'}
            }
            Record "$taskLane-$taskAction" { & wsl.exe -d Ubuntu -- env RUSTUP_AUTO_INSTALL=0 "CARGO_TARGET_DIR=/home/artur/.cache/hydracache-074-observer-control-$taskLane" /home/artur/.cargo/bin/cargo "+$taskVersion" @taskArgs }
        }
        $taskImage="/home/artur/.cache/hydracache-074-observer-control-$taskLane/release/observer-topology-profile-074"
        $taskHash=& wsl.exe -d Ubuntu -- sha256sum -- $taskImage
        if($LASTEXITCODE -ne 0){throw 'Image hash failed'}
        $taskImages[$taskLane]=$taskHash.Split(' ')[0]
    }
}catch{$taskFailure=$_.Exception.Message}
@{source=$Source;recipe_sha256=(Hash $PSCommandPath);steps=$taskSteps;images=$taskImages;failure=$taskFailure;native_gate_open=$false;server=$false;product=$false}|ConvertTo-Json -Depth 8|Set-Content -Encoding utf8NoBOM (Join-Path $taskPacket 'prepare.json')
if($taskFailure){throw $taskFailure}
$taskWsl='/mnt/c/Workspace/prj/jq/cashe/hydracache-074'
& wsl.exe -d Ubuntu -- /usr/bin/timeout --signal=TERM --kill-after=10s 330s /usr/bin/python3 -B "$taskWsl/tools/observer-topology-profile-074/run.py" --source $Source *> (Join-Path $taskPacket 'session.log')
$taskCode=[int]$LASTEXITCODE
@{source=$Source;exit=$taskCode;session_log_sha256=(Hash (Join-Path $taskPacket 'session.log'));native_gate_open=$false;server=$false;product=$false}|ConvertTo-Json|Set-Content -Encoding utf8NoBOM (Join-Path $taskPacket 'launch.json')
if($taskCode -ne 0){throw 'Synthetic session refused; no retry'}
