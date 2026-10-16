param([string]$ManifestPath = (Join-Path $PSScriptRoot 'manifest.json'), [switch]$CheckWorkingCopy)
$ErrorActionPreference='Stop'
function Check($ok,[string]$message) { if (-not $ok) { throw $message } }
function Same($a,$b,[string]$message) { Check ((@($a | Sort-Object)-join '|') -ceq (@($b | Sort-Object)-join '|')) $message }
$repo=(Resolve-Path -LiteralPath (Join-Path $PSScriptRoot '../../../../../..')).Path
$m=Get-Content -LiteralPath $ManifestPath -Raw | ConvertFrom-Json
Check ($m.format_version -eq 1 -and $m.kind -ceq 'refused-original-starting-publication' -and $m.baseline_commit -ceq 'e7513b2567a921cdc215ab003e24704fae66b17b') 'Identity differs'
Same $m.preregistration_commits @('89fb3e8fd69dbaaa2d10b2a62898050819444928','40300927221d2074bd6eaa4ab33346399bb5fb3d','6ab06bf1246169e2b1939c27162fe7e899f270db') 'Preregistration differs'
Check ($m.temporary_root -ceq '/tmp/hc074-starting-20261016.vdpJqe') 'Temporary root differs'
$yes=@('candidate_archived','runtime_rolled_back','partial_pre_ack_crashes_passed','file_and_directory_sync_reached')
$no=@('original_fault_gate_passed','candidate_accepted','successful_runtime_acknowledgement','full_crash_matrix_dispatched','lost_ack_proved','power_loss_proved','server_ready','installed_release_changed','supervisor_stopped','clock_changed','product_workload_run','qualification_changed','performance_claim')
Same @($m.scope.PSObject.Properties.Name) @($yes+$no) 'Scope inventory differs'
foreach($k in $yes) { Check ($m.scope.$k -is [bool] -and $m.scope.$k) ('Missing bounded result: '+$k) }
foreach($k in $no) { Check ($m.scope.$k -is [bool] -and -not $m.scope.$k) ('Unsupported claim: '+$k) }
$c=$m.coverage
foreach($k in @('expected_fault_interruptions_passed_per_compiler','attribution_chains_per_compiler','partial_sigkills_per_compiler','partial_fresh_inspections_per_compiler','partial_healthy_controls','full_healthy_controls','candidate_fast_units_per_compiler','candidate_public_api_tests_per_compiler','candidate_selected_compile_fail_per_compiler','candidate_regressions_per_compiler','candidate_static_contracts','restored_static_contracts','restored_registered_gates')) {
    $expected=('{"compilers":["1.94.0","1.88.0"],"expected_fault_interruptions_passed_per_compiler":8,"attribution_chains_per_compiler":2,"partial_sigkills_per_compiler":8,"partial_fresh_inspections_per_compiler":16,"partial_healthy_controls":0,"full_healthy_controls":0,"candidate_fast_units_per_compiler":10,"candidate_public_api_tests_per_compiler":2,"candidate_selected_compile_fail_per_compiler":50,"candidate_regressions_per_compiler":62,"candidate_static_contracts":185,"restored_static_contracts":184,"restored_registered_gates":177}' | ConvertFrom-Json).$k
    Check ($c.$k -eq $expected) ('Coverage differs: '+$k)
}
Same $c.compilers @('1.94.0','1.88.0') 'Compilers differ'
$keys=@('starting-publication-red-api','starting-publication-pinned-build-194','starting-publication-pinned-build-188','starting-publication-fast-194','starting-publication-fast-188','starting-publication-docs-194','starting-publication-docs-188','starting-publication-regression-194','starting-publication-regression-188','starting-publication-clippy-win194','starting-publication-clippy-win188','starting-publication-clippy-linux194','starting-publication-clippy-linux188','starting-publication-check-win194','starting-publication-check-win188','starting-publication-check-linux194','starting-publication-check-linux188','starting-publication-static-first-failure','starting-publication-static-pinned','starting-publication-fmt','starting-publication-registry','starting-publication-doccheck','starting-publication-localcontract','starting-publication-server-root-create','starting-publication-server-transfer','starting-publication-server-run','starting-publication-binary-hashes','starting-publication-attribution-build-194','starting-publication-attribution-build-188','starting-publication-attribution-hashes','starting-publication-attribution-transfer','starting-publication-attribution-run','starting-publication-partial-build-194','starting-publication-partial-build-188','starting-publication-partial-hashes','starting-publication-partial-transfer','starting-publication-partial-run','starting-publication-server-postflight','starting-publication-restored-static','starting-publication-restored-doccheck','starting-publication-restored-registry','starting-publication-restored-localcontract','starting-publication-restored-ship-guard','starting-publication-restored-real-ship-guard')
$failed=@('starting-publication-red-api','starting-publication-static-first-failure','starting-publication-server-run','starting-publication-restored-ship-guard','starting-publication-restored-real-ship-guard')
Same @($m.commands.key) $keys 'Command inventory differs'
$extras=@('run.sh','attribute.sh','partial.sh','fault-194.log','fault-188.log','attribution-194.log','attribution-188.log','partial-194.log','partial-188.log','candidate-before-attribution.patch','candidate-with-attribution.patch','candidate-with-partial-crash.patch','candidate-source-observations.json')
Same @($m.artifacts.path) @($extras+@($keys | ForEach-Object { 'logs/'+$_+'.json' })) 'Artifact inventory differs'
Check ($m.artifacts.Count -eq 57 -and $m.commands.Count -eq 44) 'Duplicate artifact/command'
foreach($a in $m.artifacts) {
    $p=Join-Path $PSScriptRoot $a.path
    Check ((Get-Item -LiteralPath $p).Length -eq $a.bytes -and (Get-FileHash -LiteralPath $p -Algorithm SHA256).Hash.ToLowerInvariant() -ceq $a.sha256) ('Artifact differs: '+$a.path)
}
$logs=@{}
foreach($cmd in $m.commands) {
    $r=Get-Content -LiteralPath (Join-Path $PSScriptRoot ('logs/'+$cmd.key+'.json')) -Raw | ConvertFrom-Json
    $expected=if($failed -ccontains $cmd.key){1}else{0}
    Check ($r.key -ceq $cmd.key -and $r.exit_code -eq $expected -and $cmd.exit_code -eq $expected -and -not [string]::IsNullOrWhiteSpace($r.command)) ('Command outcome differs: '+$cmd.key)
    $logs[$cmd.key]=$r.output
}
$stages=@('BeforeCreate','Created','CreateDirectorySynced','PrefixWritten','BytesWritten','Sealed','FileSynced','DirectorySynced')
foreach($lane in @('194','188')) {
    $fault=Get-Content -LiteralPath (Join-Path $PSScriptRoot ("fault-$lane.log")) -Raw
    $rows=[regex]::Matches($fault,'HC074-STARTING-INTERRUPT cut=(\w+) sticky=true fence_retained=true')
    Check ($rows.Count -eq 8 -and $fault.Contains('test result: FAILED. 0 passed; 1 failed')) 'Original fault failure lost'
    Same @($rows | ForEach-Object { $_.Groups[1].Value }) $stages 'Original reached cuts differ'
    $attribute=Get-Content -LiteralPath (Join-Path $PSScriptRoot ("attribution-$lane.log")) -Raw
    Check ([regex]::Matches($attribute,'acknowledged=false visited=\[BeforeCreate, Created, CreateDirectorySynced, PrefixWritten, BytesWritten, Sealed, FileSynced, DirectorySynced\] elapsed_ns=\d+ error=Original\(Causal\(Time\)\) event=None gate_complete=false').Count -eq 2) 'Refusal attribution differs'
    Check (-not $attribute.Contains('acknowledged=true') -and $attribute.Contains('test result: ok. 1 passed; 0 failed')) 'Diagnostic promoted'
    $partial=Get-Content -LiteralPath (Join-Path $PSScriptRoot ("partial-$lane.log")) -Raw
    $rows=[regex]::Matches($partial,'collector_starting_crash seed=0x074057d1 repeat=0 stage=(\w+) sigkill=true fresh_inspection=([12]) empty=false unchanged=true')
    Check ($rows.Count -eq 16) 'Partial crash coverage differs'
    foreach($stage in $stages) { foreach($inspection in @('1','2')) { Check (@($rows | Where-Object { $_.Groups[1].Value -ceq $stage -and $_.Groups[2].Value -ceq $inspection }).Count -eq 1) 'Missing exact partial crash row' } }
    Check ($partial.Contains('sigkills=8 fresh_inspections=16 controls=0 acknowledging=false after_return=false lost_ack_proved=false gate_complete=false') -and $partial.Contains('test result: ok. 1 passed; 0 failed')) 'Partial summary differs'
    Check ($logs[('starting-publication-fast-'+$lane)].Contains('10 passed; 0 failed; 2 ignored')) 'Fast candidate count differs'
    Check ($logs[('starting-publication-docs-'+$lane)].Contains('50 passed; 0 failed')) 'Compile-fail count differs'
    foreach($n in @(24,20,5,7,6)) { Check ($logs[('starting-publication-regression-'+$lane)].Contains(($n.ToString()+' passed; 0 failed'))) 'Regression count differs' }
}
Check ($logs['starting-publication-static-pinned'].Contains('185 passed; 0 failed') -and $logs['starting-publication-restored-static'].Contains('184 passed; 0 failed')) 'Static gates differ'
Check ($logs['starting-publication-restored-registry'].Contains('177 discovered gates, all registered')) 'Restored registry differs'
Check ($logs['starting-publication-restored-real-ship-guard'].Contains('ship admission is closed')) 'Real closed ship guard absent'
$post=$logs['starting-publication-server-postflight']
Check ($m.host.supervisor_pid -eq 8839 -and $m.host.restarts -eq 0 -and $m.host.uid -eq 1000 -and $m.host.gid -eq 1000 -and $m.host.boot_id -ceq 'e81671d6-bf23-4ebf-b8fe-5dad1f45866f' -and $m.host.installed_binary_sha256 -ceq 'f797629f65385ebfd4d94577a43a2d1d020f71c1e841028dad7d02a51b47ad62' -and $m.host.installed_receipt_sha256 -ceq 'b314953d4deedaeaade3e24468da4844b7f13d348a5d77b8e8f41c672ff44525') 'Host pins differ'
foreach($s in @('MainPID=8839','NRestarts=0','ActiveState=active','SubState=running','ExecMainStartTimestamp=Tue 2026-10-06 15:03:40 CEST','full_crash_not_dispatched=true',$m.host.boot_id,$m.host.installed_binary_sha256,$m.host.installed_receipt_sha256)) { Check ($post.Contains($s)) ('Postflight missing: '+$s) }
Check ($m.binaries.Count -eq 6) 'Binary inventory differs'
foreach($b in $m.binaries) {
    $script=if($b.name.StartsWith('starting-')){'run.sh'}elseif($b.name.StartsWith('attribution-')){'attribute.sh'}else{'partial.sh'}
    Check ((Get-Content -LiteralPath (Join-Path $PSScriptRoot $script) -Raw).Contains($b.sha256+'  '+$b.name)) 'Runner binary pin differs'
}
$frozen=@('crates/xtask/tests/performance_contract_074.rs','crates/xtask/tests/support/performance_contract_074_starting_record.rs','docs/COMPAT.md','docs/testing/gated-test-registry.toml','tools/long-run-supervisor-074/src/diagnostic_collector_reservation_storage.rs','tools/long-run-supervisor-074/src/diagnostic_original_start_consent.rs','tools/long-run-supervisor-074/src/diagnostic_validation_frame.rs','tools/long-run-supervisor-074/src/diagnostic_validation_frame_tests.rs')
$added=@('crates/xtask/tests/support/performance_contract_074_starting_publication.rs','tools/long-run-supervisor-074/src/diagnostic_collector_starting_storage.rs','tools/long-run-supervisor-074/src/diagnostic_collector_starting_publication.rs','tools/long-run-supervisor-074/src/diagnostic_collector_starting_publication_tests.rs','tools/long-run-supervisor-074/src/diagnostic_collector_starting_crash_tests.rs','tools/long-run-supervisor-074/tests/diagnostic_collector_starting_publication.rs')
Same @($m.baseline_runtime_blobs.path) $frozen 'Baseline inventory differs'
Same $m.added_candidate_paths $added 'Candidate inventory differs'
Check ($m.baseline_runtime_blobs.Count -eq 8 -and $m.added_candidate_paths.Count -eq 6) 'Duplicate source'
Push-Location $repo
try {
    foreach($p in $m.preregistration_commits) { & git merge-base --is-ancestor $m.baseline_commit $p; Check ($LASTEXITCODE -eq 0) 'Preregistration ancestry differs' }
    foreach($f in $m.baseline_runtime_blobs) {
        $blob=& git rev-parse ($m.baseline_commit+':'+$f.path)
        Check ($LASTEXITCODE -eq 0 -and $blob -ceq $f.git_blob) 'Baseline blob differs'
    }
    foreach($name in @('candidate-before-attribution.patch','candidate-with-attribution.patch','candidate-with-partial-crash.patch')) {
        $patch=Get-Content -LiteralPath (Join-Path $PSScriptRoot $name) -Raw
        $rows=[regex]::Matches($patch,'(?m)^diff --git a/(.+) b/[^\r\n]+')
        Same @($rows | ForEach-Object { $_.Groups[1].Value }) @($frozen+$added) 'Patch source scope differs'
        Check ($rows.Count -eq 14 -and $patch.Contains('libc::O_EXCL') -and $patch.Contains('sync_all()') -and $patch.Contains('Some(libc::SIGKILL)')) 'Patch mechanism differs'
        if($CheckWorkingCopy) { & git apply --check (Join-Path $PSScriptRoot $name); Check ($LASTEXITCODE -eq 0) 'Archived candidate does not apply' }
    }
    if($CheckWorkingCopy) {
        & git diff --quiet $m.baseline_commit -- @frozen
        Check ($LASTEXITCODE -eq 0) 'Accepted runtime differs'
        foreach($p in $added) { Check (-not (Test-Path -LiteralPath $p)) 'Candidate still active' }
    }
} finally { Pop-Location }
'Starting evidence intact: 16 pre-ack SIGKILL / 32 fresh refusals PASS; original fault gate FAILED, lost ack and server admission CLOSED'
