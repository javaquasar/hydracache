#requires -Version 7.2
param([Parameter(Mandatory=$true)][ValidatePattern('^[a-z0-9-]{1,40}$')][string]$Attempt)
$ErrorActionPreference='Stop'
$taskRepo=(Resolve-Path (Join-Path $PSScriptRoot '../..')).Path
Set-Location $taskRepo
if(git status --porcelain --untracked-files=no){throw 'Collect only clean frozen method'}
$taskMethod=git rev-parse HEAD
$taskReceipt=Get-Content ('target/authority-runtime-cpu-'+$Attempt+'/result.json') -Raw|ConvertFrom-Json
if($taskReceipt.failure -or $taskReceipt.measure_before.head -ne $taskMethod){throw 'CPU cohort failed or method changed'}
$taskPacket=Join-Path $taskRepo 'docs/testing/performance/0.74/local-runs/authority-runtime-cpu-20261022'
if(Test-Path -LiteralPath $taskPacket){throw 'Never overwrite CPU packet'}
New-Item -ItemType Directory -Path $taskPacket -ErrorAction Stop|Out-Null
foreach($taskRun in Get-ChildItem -LiteralPath (Join-Path $taskRepo 'target') -Directory|Where-Object {$_.Name.StartsWith('authority-runtime-cpu-') -and (Test-Path -LiteralPath (Join-Path $_.FullName 'result.json'))}|Sort-Object Name){
    $taskName=$taskRun.Name.Substring('authority-runtime-cpu-'.Length)
    $taskLabel=if($taskName -eq $Attempt){'measured'}else{$taskName+'-attempt'}
    Copy-Item -LiteralPath $taskRun.FullName -Destination (Join-Path $taskPacket $taskLabel) -Recurse -ErrorAction Stop
    $taskOld=Get-Content (Join-Path $taskRun.FullName 'result.json') -Raw|ConvertFrom-Json
    if($taskOld.owned_root -notmatch '^/home/artur/\.cache/hydracache-074-runtime-cpu-20261022/[a-z0-9-]{1,40}$'){throw 'Unexpected source root'}
    foreach($taskRole in @('baseline','installed')){
        $taskManifest=Get-Content (Join-Path $taskRun.FullName ($taskRole+'-source.json')) -Raw|ConvertFrom-Json
        $taskOutput=Join-Path $taskPacket ('source/overlays/'+$taskName+'/'+$taskRole)
        New-Item -ItemType Directory -Path $taskOutput -ErrorAction Stop|Out-Null
        foreach($taskRelative in @('src/diagnostic_authority_fixture_tests.rs','tests/support/runtime_cpu_probe.rs','tests/support/runtime_cpu_storage.rs')){
            $taskMember='tools/long-run-supervisor-074/'+$taskRelative
            $taskExpected=@($taskManifest|Where-Object path -eq $taskMember)
            if($taskExpected.Count -ne 1){throw 'CPU overlay membership refused'}
            $taskOriginal='\\wsl.localhost\Ubuntu'+$taskOld.owned_root.Replace('/','\')+'\'+$taskRole+'\'+$taskMember.Replace('/','\')
            if((Get-FileHash -LiteralPath $taskOriginal).Hash.ToLowerInvariant() -ne $taskExpected[0].sha256){throw 'CPU overlay source changed'}
            Copy-Item -LiteralPath $taskOriginal -Destination (Join-Path $taskOutput (Split-Path $taskRelative -Leaf)) -ErrorAction Stop
        }
    }
}
$taskDevelopment=Join-Path $taskPacket 'development'
New-Item -ItemType Directory -Path $taskDevelopment -ErrorAction Stop|Out-Null
Get-ChildItem -LiteralPath (Join-Path $taskRepo 'target/authority-runtime-cpu-development') -File|Where-Object {$_.Extension -in @('.log','.json')}|
    ForEach-Object {Copy-Item -LiteralPath $_.FullName -Destination (Join-Path $taskDevelopment $_.Name) -ErrorAction Stop}
$taskSource=Join-Path $taskPacket 'source'
$taskArchive=Join-Path $taskSource 'method.zip'
git --no-replace-objects archive --format=zip --output=$taskArchive $taskMethod -- tools/authority-runtime-cpu-074 tools/authority-runtime-allocation-074/run.py docs/testing/performance/0.74/authority-runtime-cpu-plan-20261022.md
if($LASTEXITCODE -ne 0){throw 'CPU method archive failed'}
Expand-Archive -LiteralPath $taskArchive -DestinationPath (Join-Path $taskSource 'method') -ErrorAction Stop
$taskChanges=@(git diff --name-only 33c54b0cda8c330b115009cdf092ad002077cedf 66d5f371680d085a24de8f2558e1f049c71b9e7b -- tools/long-run-supervisor-074)
if($LASTEXITCODE -ne 0 -or $taskChanges.Count -ne 8){throw 'CPU runtime delta changed'}
foreach($taskRole in @(@('baseline','33c54b0cda8c330b115009cdf092ad002077cedf'),@('installed','66d5f371680d085a24de8f2558e1f049c71b9e7b'))){
    $taskArchive=Join-Path $taskSource ($taskRole[0]+'.zip')
    git --no-replace-objects archive --format=zip --output=$taskArchive $taskRole[1] -- Cargo.toml Cargo.lock @taskChanges
    if($LASTEXITCODE -ne 0){throw 'CPU runtime input archive failed'}
}
@{schema='authority-runtime-cpu-context-074-v1';method=$taskMethod;runtime_sources=$taskReceipt.runtime_sources;predecessor='496e3628c1975e5e006c1b8a12ecef8211314c3367cff1d0d50b40c7846f9913';runtime_changed=$false;live=$false;server=$false;product=$false}|ConvertTo-Json -Depth 5|Out-File (Join-Path $taskPacket 'context.json') -Encoding utf8 -NoClobber
python -B tools/authority-runtime-cpu-074/seal.py $taskPacket --seal
if($LASTEXITCODE -ne 0){throw 'CPU first seal refused; preserve packet'}
