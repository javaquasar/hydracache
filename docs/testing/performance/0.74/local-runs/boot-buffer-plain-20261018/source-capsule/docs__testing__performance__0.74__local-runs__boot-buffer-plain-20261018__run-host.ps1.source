$ErrorActionPreference='Stop'
$repo=(Resolve-Path (Join-Path $PSScriptRoot '../../../../../..')).Path
$out=Join-Path $repo 'target/boot-buffer-plain-20261018'
if (Test-Path -LiteralPath (Join-Path $out 'host-location.json')) {throw 'no duplicate host dispatch'}
$files=@()
foreach ($compiler in @('194','188')) {
    foreach ($role in @('Baseline','Candidate')) {
        $entry=Get-Content -LiteralPath (Join-Path $out "$role-$compiler.json") -Raw | ConvertFrom-Json
        $local=Join-Path $out "tests-$role-$compiler"
        if ($entry.role -cne $role -or $entry.compiler -cne $compiler -or (Get-FileHash -LiteralPath $local -Algorithm SHA256).Hash.ToLowerInvariant() -cne $entry.sha256) {throw 'image binding'}
        $files+=@{local=$local;remote="tests-$role-$compiler"}
    }
}
$files+=@{local=(Join-Path $PSScriptRoot 'host-guard.sh');remote='guard.sh'}
$files+=@{local=(Join-Path $PSScriptRoot 'host-run.sh');remote='run.sh'}
foreach ($name in @('timing-controls-074','build-receipt-v1.json','build-log.jsonl','Cargo.lock.root','Cargo.lock.observer','embedded.json','direct.json','resp2.json','resp3.json')) {
    $files+=@{local=(Join-Path $repo "target/diagnostic-start-material-bundle-20261017/$name");remote="bundle/$name"}
}
$inventory=@($files | ForEach-Object {@{path=$_.remote;sha256=(Get-FileHash -LiteralPath $_.local -Algorithm SHA256).Hash.ToLowerInvariant()}})
$inventory | ConvertTo-Json | Set-Content -Encoding utf8NoBOM (Join-Path $out 'upload-inputs.json')
$sum=($inventory | ForEach-Object {"$($_.sha256)  $($_.path)"}) -join "`n"
[IO.File]::WriteAllText((Join-Path $out 'SHA256SUMS'),$sum+"`n",[Text.UTF8Encoding]::new($false))
$remote=(& ssh -o BatchMode=yes -o StrictHostKeyChecking=yes -o ConnectTimeout=8 hydracache-ax42 'umask 077; mktemp -d /tmp/hc074-boot-buffer-plain-20261018.XXXXXX') -join "`n"
if ($LASTEXITCODE -ne 0 -or $remote -cnotmatch '^/tmp/hc074-boot-buffer-plain-20261018\.[A-Za-z0-9]{6}$') {throw 'remote root'}
@{root=$remote;installed=$false;supervisor_changed=$false} | ConvertTo-Json | Set-Content -Encoding utf8NoBOM (Join-Path $out 'host-location.json')
ssh -o BatchMode=yes -o StrictHostKeyChecking=yes hydracache-ax42 "set -eu; umask 077; mkdir -- '$remote/bundle'; chmod 700 -- '$remote/bundle'"
if ($LASTEXITCODE -ne 0) {throw 'bundle directory'}
foreach ($file in $files) {
    scp -o BatchMode=yes -o StrictHostKeyChecking=yes -- $file.local "hydracache-ax42:${remote}/$($file.remote)"
    if ($LASTEXITCODE -ne 0) {throw 'upload'}
}
scp -o BatchMode=yes -o StrictHostKeyChecking=yes -- (Join-Path $out 'SHA256SUMS') "hydracache-ax42:${remote}/SHA256SUMS"
if ($LASTEXITCODE -ne 0) {throw 'sum upload'}
ssh -o BatchMode=yes -o StrictHostKeyChecking=yes hydracache-ax42 "set -eu; umask 077; chmod 500 -- '$remote/tests-Baseline-194' '$remote/tests-Candidate-194' '$remote/tests-Baseline-188' '$remote/tests-Candidate-188'; cd -- '$remote'; . ./guard.sh; no_fixture; sha256sum -c SHA256SUMS" 2>&1 | Tee-Object -FilePath (Join-Path $out 'host-preflight.log')
if ($LASTEXITCODE -ne 0) {throw 'initial host preflight'}
ssh -o BatchMode=yes -o StrictHostKeyChecking=yes hydracache-ax42 "bash '$remote/run.sh' '$remote'" 2>&1 | Tee-Object -FilePath (Join-Path $out 'host.log')
$runExit=$LASTEXITCODE
ssh -o BatchMode=yes -o StrictHostKeyChecking=yes hydracache-ax42 "cd -- '$remote'; . ./guard.sh; no_fixture; sha256sum -c SHA256SUMS; printf 'HC074-PLAIN-INDEPENDENT-POSTFLIGHT complete=true supervisor_unchanged=true no_fixture=true\n'" 2>&1 | Tee-Object -FilePath (Join-Path $out 'host-postflight.log')
$postExit=$LASTEXITCODE
@{run_exit=$runExit;post_exit=$postExit;retry=$false} | ConvertTo-Json | Set-Content -Encoding utf8NoBOM (Join-Path $out 'host-exits.json')
if ($postExit -ne 0) {throw 'independent postflight'}
if ($runExit -ne 0) {throw "host fixture failed $runExit"}
