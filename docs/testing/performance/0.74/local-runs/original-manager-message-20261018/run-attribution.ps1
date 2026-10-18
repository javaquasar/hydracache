$ErrorActionPreference='Stop'
$location=Get-Content (Join-Path $PSScriptRoot 'host-location.json') -Raw | ConvertFrom-Json
$remote=[string]$location.root
if ($remote -cnotmatch '^/tmp/hc074-original-manager-message-20261018\.[A-Za-z0-9]{6}$') {throw 'remote path'}
$raw=Get-Content (Join-Path $PSScriptRoot 'attribution-build.jsonl') | Where-Object {$_.StartsWith('{')} | ForEach-Object {$_ | ConvertFrom-Json}
$artifacts=@($raw | Where-Object {$_.reason -ceq 'compiler-artifact' -and $_.target.name -ceq 'hydracache_long_run_supervisor_074' -and $_.profile.test -and $_.executable})
if ($artifacts.Count -ne 1) {throw 'attribution image'}
$image=[string]$artifacts[0].executable
$hash=(& wsl -d Ubuntu -- sha256sum -- $image) -join "`n"
if ($LASTEXITCODE -ne 0 -or $hash -cnotmatch '^([a-f0-9]{64})  ') {throw 'attribution hash'}
$sha=$Matches[1]
$local='C:/Workspace/prj/jq/cashe/hydracache-074/target/original-manager-message-20261018/tests-attribution-194'
wsl -d Ubuntu -- cp -- $image /mnt/c/Workspace/prj/jq/cashe/hydracache-074/target/original-manager-message-20261018/tests-attribution-194
if ($LASTEXITCODE -ne 0 -or (Get-FileHash -LiteralPath $local -Algorithm SHA256).Hash.ToLowerInvariant() -cne $sha) {throw 'attribution copy'}
@{image=$image;sha256=$sha;compiler='194';purpose='separate cost attribution, not acceptance'} | ConvertTo-Json | Set-Content -Encoding utf8NoBOM (Join-Path $PSScriptRoot 'attribution-image.json')
ssh -o BatchMode=yes -o StrictHostKeyChecking=yes hydracache-ax42 "cd -- '$remote'; . ./guard.sh; no_fixture; test ! -e ./tests-attribution-194"
if ($LASTEXITCODE -ne 0) {throw 'attribution preflight'}
scp -o BatchMode=yes -o StrictHostKeyChecking=yes -- $local "hydracache-ax42:${remote}/tests-attribution-194"
if ($LASTEXITCODE -ne 0) {throw 'attribution upload'}
$test='diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::tests::fence_preflight_tests::attempt_tests::collector_tests::observed_reservation_tests::collector_reserved_tests::progress_tests::start_consent_tests::validation_frame_tests::starting_tests::sink_tests::start_material_tests::pipe_transport_tests::safety_alarm_tests::fixed_grouped_tests::custody_tests::descriptor_plan_tests::barred_custody_tests::collector_tests::enrollment_tests::manager_tests::addressed_tests::original_addressed_round_cost_attribution'
ssh -o BatchMode=yes -o StrictHostKeyChecking=yes hydracache-ax42 "cd -- '$remote'; . ./guard.sh; no_fixture; chmod 500 ./tests-attribution-194; sha256sum ./tests-attribution-194; HC074_START_MATERIAL_BUNDLE='$remote/bundle' ./tests-attribution-194 '$test' --exact --ignored --nocapture --test-threads=1" 2>&1 | Tee-Object -FilePath (Join-Path $PSScriptRoot 'attribution.log')
$runExit=$LASTEXITCODE
ssh -o BatchMode=yes -o StrictHostKeyChecking=yes hydracache-ax42 "cd -- '$remote'; . ./guard.sh; no_fixture; sha256sum ./tests-194 ./tests-188" 2>&1 | Tee-Object -FilePath (Join-Path $PSScriptRoot 'attribution-postflight.log')
if ($LASTEXITCODE -ne 0) {throw 'attribution postflight'}
if ($runExit -ne 0) {throw "attribution exit $runExit (never acceptance)"}
'HC074-ADDRESSED-ATTRIBUTION complete=true selected_cohort_healed=false admission=false'
