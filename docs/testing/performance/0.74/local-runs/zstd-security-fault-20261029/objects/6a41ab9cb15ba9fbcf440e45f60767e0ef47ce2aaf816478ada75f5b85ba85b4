#requires -Version 7.2
$ErrorActionPreference='Stop'
$taskRepo=(Resolve-Path (Join-Path $PSScriptRoot '../..')).Path
Set-Location $taskRepo
$taskPacket=Join-Path $taskRepo 'docs/testing/performance/0.74/local-runs/authority-runtime-20261022'
if(Test-Path -LiteralPath $taskPacket){throw 'Never replace runtime evidence'}
$taskSource=git rev-parse HEAD
if($LASTEXITCODE -ne 0){throw 'Missing committed method'}
if(git status --porcelain --untracked-files=no){throw 'Collect only the clean frozen method'}
$taskPrior=Join-Path $taskRepo 'docs/testing/performance/0.74/local-runs/authority-rust-history-20261022'
$taskPriorInventory=Join-Path $taskPrior 'inventory.json'
if((Get-FileHash -LiteralPath $taskPriorInventory -Algorithm SHA256).Hash.ToLowerInvariant() -ne '4904ede1ef28f2e4d96b1da0f5be8b633c20a3d2fbb6524289567b974581b63f'){throw 'Prior seal changed'}
$taskPriorRows=(Get-Content -LiteralPath $taskPriorInventory -Raw|ConvertFrom-Json).files
if($taskPriorRows.Count -ne 61){throw 'Prior inventory count changed'}
foreach($taskRow in $taskPriorRows){
    $taskFile=Join-Path $taskPrior $taskRow.path
    if((Get-Item -LiteralPath $taskFile).Length -ne $taskRow.bytes -or (Get-FileHash -LiteralPath $taskFile -Algorithm SHA256).Hash.ToLowerInvariant() -ne $taskRow.sha256){throw "Prior member changed: $($taskRow.path)"}
}
New-Item -ItemType Directory -Path $taskPacket -ErrorAction Stop|Out-Null
Copy-Item -LiteralPath $taskPriorInventory -Destination (Join-Path $taskPacket 'predecessor-inventory.json')
foreach($taskPair in @(
    @('target/authority-runtime-baseline','baseline'),
    @('target/authority-runtime-development','development'),
    @('target/authority-runtime-dev-194','driver-first-refusal'),
    @('target/authority-runtime-dev-194-path-fixed','linux-development-194'),
    @('target/authority-runtime-final-194','linux-194'),
    @('target/authority-runtime-final-188','linux-188'),
    @('target/authority-runtime-first-checks','windows')
)){
    Copy-Item -LiteralPath (Join-Path $taskRepo $taskPair[0]) -Destination (Join-Path $taskPacket $taskPair[1]) -Recurse -ErrorAction Stop
}
$taskPaths=@(git diff --name-only acd44d86daa6d55490a2565c9d0209f5775922a4 $taskSource -- crates/xtask/tests tools/authority-runtime-074 tools/long-run-supervisor-074)
if($LASTEXITCODE -ne 0 -or $taskPaths.Count -lt 50){throw 'Incomplete method snapshot list'}
$taskPaths+='docs/testing/performance/0.74/authority-runtime-plan-20261022.md'
$taskArchive=Join-Path $taskPacket 'source.zip'
git --no-replace-objects archive --format=zip --output=$taskArchive $taskSource -- @taskPaths
if($LASTEXITCODE -ne 0){throw 'Source archive failed'}
Expand-Archive -LiteralPath $taskArchive -DestinationPath (Join-Path $taskPacket 'source') -ErrorAction Stop
@{source=$taskSource;preregistration='acd44d86daa6d55490a2565c9d0209f5775922a4';artifact='f193fabae86c9bd7a2e778a9463a43984834deb5';predecessor_verified_members=61;source_paths=$taskPaths;server=$false;product=$false;clocks_changed=$false;services_changed=$false}|ConvertTo-Json -Depth 5|Out-File (Join-Path $taskPacket 'context.json') -Encoding utf8 -NoClobber
python -B tools/authority-runtime-074/review.py $taskPacket --seal
if($LASTEXITCODE -ne 0){throw 'Runtime evidence sealing refused; preserve first packet'}
