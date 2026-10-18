$ErrorActionPreference='Stop'
$root=(Resolve-Path (Join-Path $PSScriptRoot '../../../../../..')).Path
$images=Get-Content (Join-Path $PSScriptRoot 'images.json') -Raw | ConvertFrom-Json
$remote=(& ssh -o BatchMode=yes -o StrictHostKeyChecking=yes -o ConnectTimeout=8 hydracache-ax42 'umask 077; mktemp -d /tmp/hc074-original-live-walk-20261018.XXXXXX') -join "`n"
if ($LASTEXITCODE -ne 0 -or $remote -cnotmatch '^/tmp/hc074-original-live-walk-20261018\.[A-Za-z0-9]{6}$') {throw 'unsafe remote root'}
@{root=$remote;purpose='nonroot original live walk correctness fixtures';installed=$false} | ConvertTo-Json | Set-Content -Encoding utf8NoBOM (Join-Path $PSScriptRoot 'host-location.json')
$artifact=Join-Path $root 'target/original-live-walk-20261018'
New-Item -ItemType Directory -Path $artifact -Force | Out-Null
foreach ($entry in $images) {
    if ($entry.compiler -cnotin @('194','188') -or $entry.image -cnotmatch '^/home/artur/\.cache/hydracache-074-(194|188)-check/release/deps/hydracache_long_run_supervisor_074-[a-f0-9]{16}$') {throw 'image path'}
    $image=Join-Path $artifact "tests-$($entry.compiler)"
    wsl -d Ubuntu -- cp -- $entry.image "/mnt/c/Workspace/prj/jq/cashe/hydracache-074/target/original-live-walk-20261018/tests-$($entry.compiler)"
    if ($LASTEXITCODE -ne 0 -or (Get-FileHash -LiteralPath $image -Algorithm SHA256).Hash.ToLowerInvariant() -cne $entry.sha256) {throw 'image copy'}
    scp -o BatchMode=yes -o StrictHostKeyChecking=yes -- $image "hydracache-ax42:${remote}/tests-$($entry.compiler)"
    if ($LASTEXITCODE -ne 0) {throw 'image upload'}
}
foreach ($pair in @(@('host-guard.sh','guard.sh'),@('host-run.sh','run.sh'))) {
    scp -o BatchMode=yes -o StrictHostKeyChecking=yes -- (Join-Path $PSScriptRoot $pair[0]) "hydracache-ax42:${remote}/$($pair[1])"
    if ($LASTEXITCODE -ne 0) {throw 'script upload'}
}
ssh -o BatchMode=yes -o StrictHostKeyChecking=yes hydracache-ax42 "chmod 500 -- '$remote/tests-194' '$remote/tests-188'; cd -- '$remote'; . ./guard.sh; no_fixture" 2>&1 | Tee-Object -FilePath (Join-Path $PSScriptRoot 'host-preflight.log')
if ($LASTEXITCODE -ne 0) {throw 'initial host guard'}
ssh -o BatchMode=yes -o StrictHostKeyChecking=yes hydracache-ax42 "bash '$remote/run.sh' '$remote'" 2>&1 | Tee-Object -FilePath (Join-Path $PSScriptRoot 'host.log')
$runExit=$LASTEXITCODE
ssh -o BatchMode=yes -o StrictHostKeyChecking=yes hydracache-ax42 "cd -- '$remote'; . ./guard.sh; no_fixture" 2>&1 | Tee-Object -FilePath (Join-Path $PSScriptRoot 'host-postflight.log')
if ($LASTEXITCODE -ne 0) {throw 'host postflight'}
if ($runExit -ne 0) {throw "host fixture exit $runExit"}
'HC074-LIVE-WALK-DISPATCH complete=true supervisor_unchanged=true start=false'
