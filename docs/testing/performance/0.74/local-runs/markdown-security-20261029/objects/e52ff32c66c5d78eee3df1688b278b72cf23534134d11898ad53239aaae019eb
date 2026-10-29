#requires -Version 7.2
$ErrorActionPreference='Stop'
$taskRepo=(Resolve-Path (Join-Path $PSScriptRoot '../..')).Path
Set-Location $taskRepo
$taskPacket=Join-Path $taskRepo 'docs/testing/performance/0.74/local-runs/authority-runtime-allocation-20261022'
if(Test-Path -LiteralPath $taskPacket){throw 'Never overwrite allocation evidence'}
if(git status --porcelain --untracked-files=no){throw 'Collect the frozen clean measurement method'}
$taskReceipt=Get-Content target/authority-runtime-allocation-seventh/result.json -Raw|ConvertFrom-Json
if($taskReceipt.failure -or (git rev-parse HEAD) -ne $taskReceipt.measure_before.head){throw 'Measurement failed or method changed'}
New-Item -ItemType Directory -Path $taskPacket -ErrorAction Stop|Out-Null
foreach($taskPair in @(
    @('target/authority-runtime-allocation-first','first-attempt'),
    @('target/authority-runtime-allocation-second','second-attempt'),
    @('target/authority-runtime-allocation-third','third-attempt'),
    @('target/authority-runtime-allocation-fourth','fourth-attempt'),
    @('target/authority-runtime-allocation-fifth','fifth-attempt'),
    @('target/authority-runtime-allocation-sixth','sixth-attempt'),
    @('target/authority-runtime-allocation-seventh','measured')
)){
    Copy-Item -LiteralPath (Join-Path $taskRepo $taskPair[0]) -Destination (Join-Path $taskPacket $taskPair[1]) -Recurse -ErrorAction Stop
}
$taskDevelopment=Join-Path $taskPacket 'development'
New-Item -ItemType Directory -Path $taskDevelopment -ErrorAction Stop|Out-Null
Get-ChildItem -LiteralPath (Join-Path $taskRepo 'target/authority-runtime-allocation-development') -File|
    Where-Object {$_.Extension -in @('.log','.rs')}|
    ForEach-Object {Copy-Item -LiteralPath $_.FullName -Destination (Join-Path $taskDevelopment $_.Name) -ErrorAction Stop}
$taskMethod=git rev-parse HEAD
$taskSource=Join-Path $taskPacket 'source'
New-Item -ItemType Directory -Path $taskSource -ErrorAction Stop|Out-Null
foreach($taskAttempt in @(@('first','first-attempt'),@('second','second-attempt'),@('third','third-attempt'),@('fourth','fourth-attempt'),@('fifth','fifth-attempt'),@('sixth','sixth-attempt'),@('seventh','measured'))){
    foreach($taskRoleName in @('baseline','installed')){
        $taskManifest=Get-Content (Join-Path $taskPacket ($taskAttempt[1]+'/'+$taskRoleName+'-source.json')) -Raw|ConvertFrom-Json
        $taskRetained=Join-Path $taskSource ('development-overlays/'+$taskAttempt[0]+'/'+$taskRoleName)
        New-Item -ItemType Directory -Path $taskRetained -ErrorAction Stop|Out-Null
        foreach($taskRelative in @('src/diagnostic_authority_fixture_tests.rs','tests/support/runtime_allocation_probe.rs','tests/support/runtime_allocation_counter.rs','tests/support/runtime_allocation_storage.rs')){
            $taskMember='tools/long-run-supervisor-074/'+$taskRelative
            $taskExpected=@($taskManifest|Where-Object path -eq $taskMember)
            if($taskExpected.Count -ne 1){throw 'Overlay source membership refused'}
            $taskOriginal='\\wsl.localhost\Ubuntu\home\artur\.cache\hydracache-074-runtime-allocation-20261022\'+$taskAttempt[0]+'\'+$taskRoleName+'\'+$taskMember.Replace('/','\')
            if((Get-FileHash -LiteralPath $taskOriginal -Algorithm SHA256).Hash.ToLowerInvariant() -ne $taskExpected[0].sha256){throw 'Original overlay source changed'}
            Copy-Item -LiteralPath $taskOriginal -Destination (Join-Path $taskRetained (Split-Path $taskRelative -Leaf)) -ErrorAction Stop
        }
    }
}
$taskMethodArchive=Join-Path $taskSource 'method.zip'
git --no-replace-objects archive --format=zip --output=$taskMethodArchive $taskMethod -- tools/authority-runtime-allocation-074 docs/testing/performance/0.74/authority-runtime-allocation-plan-20261022.md
if($LASTEXITCODE -ne 0){throw 'Method archive failed'}
$taskChanges=@(git diff --name-only 33c54b0cda8c330b115009cdf092ad002077cedf 66d5f371680d085a24de8f2558e1f049c71b9e7b -- tools/long-run-supervisor-074)
if($LASTEXITCODE -ne 0 -or $taskChanges.Count -ne 8){throw 'Runtime delta changed'}
foreach($taskRole in @(@('baseline','33c54b0cda8c330b115009cdf092ad002077cedf'),@('installed','66d5f371680d085a24de8f2558e1f049c71b9e7b'))){
    $taskArchive=Join-Path $taskSource ($taskRole[0]+'.zip')
    git --no-replace-objects archive --format=zip --output=$taskArchive $taskRole[1] -- Cargo.toml Cargo.lock @taskChanges
    if($LASTEXITCODE -ne 0){throw 'Runtime input archive failed'}
}
Expand-Archive -LiteralPath (Join-Path $taskSource 'method.zip') -DestinationPath (Join-Path $taskSource 'method') -ErrorAction Stop
@{schema='authority-runtime-allocation-context-074-v1';method=$taskMethod;predecessor='fafe2b63fde717f1f34d3ad62b06a50014c9653916a8a8b79577fd9f91568375';runtime_sources=$taskReceipt.runtime_sources;no_runtime_edits=$true;server=$false;product=$false;live=$false}|ConvertTo-Json -Depth 5|Out-File (Join-Path $taskPacket 'context.json') -Encoding utf8 -NoClobber
python -B tools/authority-runtime-allocation-074/seal.py $taskPacket --seal
if($LASTEXITCODE -ne 0){throw 'First allocation seal refused; preserve packet'}
