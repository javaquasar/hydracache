$ErrorActionPreference='Stop'
$root=(Resolve-Path (Join-Path $PSScriptRoot '../../../../../..')).Path
$images=Get-Content (Join-Path $PSScriptRoot 'Candidate-images.json') -Raw | ConvertFrom-Json
$remote=(& ssh -o BatchMode=yes -o StrictHostKeyChecking=yes -o ConnectTimeout=8 hydracache-ax42 'umask 077; mktemp -d /tmp/hc074-original-preparation-budget-20261018.XXXXXX') -join "`n"
if ($LASTEXITCODE -ne 0 -or $remote -cnotmatch '^/tmp/hc074-original-preparation-budget-20261018\.[A-Za-z0-9]{6}$') {throw 'unsafe remote root'}
@{root=$remote;purpose='nonroot isolated correctness fixtures';installed=$false} | ConvertTo-Json | Set-Content -Encoding utf8NoBOM (Join-Path $PSScriptRoot 'host-location.json')
$artifact=Join-Path $root 'target/original-preparation-budget-20261018'
New-Item -ItemType Directory -Path $artifact -Force | Out-Null
foreach ($entry in $images) {
    if ($entry.compiler -cnotin @('194','188') -or $entry.image -cnotmatch '^/home/artur/\.cache/hydracache-074-(194|188)-check/release/deps/hydracache_long_run_supervisor_074-[a-f0-9]{16}$') {throw 'image path'}
    $image=Join-Path $artifact "tests-$($entry.compiler)"
    wsl -d Ubuntu -- cp -- $entry.image "/mnt/c/Workspace/prj/jq/cashe/hydracache-074/target/original-preparation-budget-20261018/tests-$($entry.compiler)"
    if ($LASTEXITCODE -ne 0 -or (Get-FileHash -LiteralPath $image -Algorithm SHA256).Hash.ToLowerInvariant() -cne $entry.sha256) {throw 'image copy'}
    scp -o BatchMode=yes -o StrictHostKeyChecking=yes -- $image "hydracache-ax42:${remote}/tests-$($entry.compiler)"
    if ($LASTEXITCODE -ne 0) {throw 'image upload'}
}
scp -o BatchMode=yes -o StrictHostKeyChecking=yes -- (Join-Path $PSScriptRoot 'host-guard.sh') "hydracache-ax42:${remote}/guard.sh"
if ($LASTEXITCODE -ne 0) {throw 'guard upload'}
scp -o BatchMode=yes -o StrictHostKeyChecking=yes -- (Join-Path $PSScriptRoot 'host-run.sh') "hydracache-ax42:${remote}/run.sh"
if ($LASTEXITCODE -ne 0) {throw 'run upload'}
ssh -o BatchMode=yes -o StrictHostKeyChecking=yes hydracache-ax42 "mkdir -- '$remote/bundle'; chmod 500 -- '$remote/tests-194' '$remote/tests-188'; cd -- '$remote'; . ./guard.sh; no_fixture"
if ($LASTEXITCODE -ne 0) {throw 'initial host guard'}
$names=@('timing-controls-074','build-receipt-v1.json','build-log.jsonl','Cargo.lock.root','Cargo.lock.observer','embedded.json','direct.json','resp2.json','resp3.json')
$files=@($names | ForEach-Object {Join-Path $root "target/diagnostic-start-material-bundle-20261017/$_"})
$files | ForEach-Object {@{name=[IO.Path]::GetFileName($_);sha256=(Get-FileHash -LiteralPath $_ -Algorithm SHA256).Hash.ToLowerInvariant()}} | ConvertTo-Json | Set-Content -Encoding utf8NoBOM (Join-Path $PSScriptRoot 'bundle-inputs.json')
scp -o BatchMode=yes -o StrictHostKeyChecking=yes -- @files "hydracache-ax42:${remote}/bundle/"
if ($LASTEXITCODE -ne 0) {throw 'bundle upload'}
ssh -o BatchMode=yes -o StrictHostKeyChecking=yes hydracache-ax42 "bash '$remote/run.sh' '$remote'" 2>&1 | Tee-Object -FilePath (Join-Path $PSScriptRoot 'host.log')
$runExit=$LASTEXITCODE
ssh -o BatchMode=yes -o StrictHostKeyChecking=yes hydracache-ax42 "cd -- '$remote'; . ./guard.sh; no_fixture" 2>&1 | Tee-Object -FilePath (Join-Path $PSScriptRoot 'host-postflight.log')
if ($LASTEXITCODE -ne 0) {throw 'host postflight'}
if ($runExit -ne 0) {throw "host fixture exit $runExit"}
'HC074-ORIGINAL-MANAGER-HOST-DISPATCH complete=true supervisor_unchanged=true start=false'
