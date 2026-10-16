param([string]$ManifestPath=(Join-Path $PSScriptRoot 'manifest.json'))
$ErrorActionPreference='Stop'
function Check($ok,[string]$why){if(-not $ok){throw $why}}
function Same($a,$b,[string]$why){Check ((@($a|Sort-Object)-join '|') -ceq (@($b|Sort-Object)-join '|')) $why}
$m=Get-Content -LiteralPath $ManifestPath -Raw|ConvertFrom-Json
Check ($m.format_version -eq 1 -and $m.kind -ceq 'original-starting-framed-process-loss-proof') 'Identity differs'
Check ($m.baseline_commit -ceq '77b3ad15c8f56cd1ef41c02e31c1bad622126a55' -and $m.temporary_root -ceq '/tmp/hc074-starting-framed-20261016.dAn2pE' -and $m.source_commit -match '^[0-9a-f]{40}$') 'Source/root differs'
$yes=@('original_fault_gate_passed','full_crash_matrix_passed','runtime_acknowledgement_passed','runtime_lost_ack_refuses_recovery','file_and_directory_sync_before_ack','original_guards_preserved','deadlines_unchanged')
$no=@('external_wire_ack_tested','power_loss_proved','reboot_proved','storage_eio_proved','installed_release_changed','supervisor_stopped','clock_changed','product_workload_run','qualification_changed','performance_claim','production_trust_enrolled','product_output_or_execution_started','all_writers_revoked','completion_or_later_cells','server_ready')
Same @($m.scope.PSObject.Properties.Name) @($yes+$no) 'Scope inventory differs'
foreach($k in $yes){Check ($m.scope.$k -is [bool] -and $m.scope.$k) ('Missing bounded result: '+$k)}
foreach($k in $no){Check ($m.scope.$k -is [bool] -and -not $m.scope.$k) ('Unsupported claim: '+$k)}
Same $m.coverage.compilers @('1.94.0','1.88.0') 'Compiler inventory differs'
$expected='{"fault_interruptions_per_compiler":9,"drift_cases_per_compiler":9,"healthy_fault_controls_per_compiler":1,"sigkills_per_compiler":30,"lost_runtime_ack_kills_per_compiler":3,"uninterrupted_crash_controls_per_compiler":3,"fresh_inspections_per_compiler":66,"repetitions":3,"fast_units_per_compiler":10,"public_api_tests_per_compiler":2,"compile_fail_per_compiler":50,"regressions_per_compiler":62,"static_contracts":186,"registered_gates":179}'|ConvertFrom-Json
foreach($p in $expected.PSObject.Properties){Check ($m.coverage.($p.Name) -eq $p.Value) ('Coverage differs: '+$p.Name)}
$keys=@('starting-framed-red-api','starting-framed-build-194','starting-framed-build-188','starting-framed-api','starting-framed-fast-194','starting-framed-fast-188','starting-framed-docs-194','starting-framed-docs-188','starting-framed-regression-194','starting-framed-regression-188','starting-framed-check-linux194','starting-framed-check-linux188','starting-framed-check-win194','starting-framed-check-win188','starting-framed-clippy-linux194','starting-framed-clippy-linux188','starting-framed-clippy-win194','starting-framed-clippy-win188','starting-framed-static','starting-framed-static-focused','starting-framed-static-final','starting-framed-static-msrv','starting-framed-fmt','starting-framed-fmt-linux','starting-framed-registry','starting-framed-doccheck','starting-framed-localcontract','starting-framed-ship-guard','starting-framed-server-root-create','starting-framed-binary-hashes','starting-framed-transfer','starting-framed-preconsent-docs-194','starting-framed-preconsent-docs-188','starting-framed-preconsent-fast-194','starting-framed-preconsent-fast-188','starting-framed-midflight','starting-framed-server-run','starting-framed-postflight')
$extras=@('run.sh','fault-194.log','fault-188.log','crash-194.log','crash-188.log','verifier-selfcheck.json','verifier-before-selfcheck.ps1')
Same @($m.artifacts.path) @($extras+@($keys|ForEach-Object{'logs/'+$_+'.json'})) 'Artifact inventory differs'
Check ($m.artifacts.Count -eq 45 -and @($m.artifacts.path|Select-Object -Unique).Count -eq 45) 'Duplicate artifact'
foreach($a in $m.artifacts){
 Check ($a.path -notmatch '(^/|\.\.|\\|:)') 'Unsafe artifact path'
 $p=Join-Path $PSScriptRoot $a.path
 Check ((Get-Item -LiteralPath $p).Length -eq $a.bytes -and (Get-FileHash -LiteralPath $p -Algorithm SHA256).Hash.ToLowerInvariant() -ceq $a.sha256) ('Artifact differs: '+$a.path)
}
$logs=@{}
foreach($k in $keys){
 $r=Get-Content -LiteralPath (Join-Path $PSScriptRoot ('logs/'+$k+'.json')) -Raw|ConvertFrom-Json
 $exit=if($k -in @('starting-framed-red-api','starting-framed-static','starting-framed-fmt','starting-framed-ship-guard')){1}else{0}
 Check ($r.key -ceq $k -and $r.exit_code -eq $exit -and -not [string]::IsNullOrWhiteSpace($r.command)) ('Command differs: '+$k)
 $logs[$k]=$r.output
}
Check ($logs['starting-framed-red-api'].Contains('E0599') -and $logs['starting-framed-static'].Contains('185 passed; 1 failed') -and $logs['starting-framed-fmt'].Contains('os error 206')) 'Red/assembly observations lost'
Check ($logs['starting-framed-static-final'].Contains('186 passed; 0 failed') -and $logs['starting-framed-registry'].Contains('179 discovered gates, all registered')) 'Static/registry differs'
Check ($logs['starting-framed-static-msrv'].Contains('186 passed; 0 failed')) 'MSRV static gate differs'
Check ($logs['starting-framed-ship-guard'].Contains('ship admission is closed')) 'Closed ship guard absent'
$stages=@('BeforeCreate','Created','CreateDirectorySynced','PrefixWritten','BytesWritten','Sealed','FileSynced','DirectorySynced','Acknowledging')
foreach($lane in @('194','188')){
 $fault=Get-Content -LiteralPath (Join-Path $PSScriptRoot ("fault-$lane.log")) -Raw
 $rows=[regex]::Matches($fault,'HC074-STARTING-INTERRUPT cut=(\w+) sticky=true fence_retained=true')
 Check ($rows.Count -eq 9) 'Interruption count differs'
 Same @($rows|ForEach-Object{$_.Groups[1].Value}) $stages 'Missing interruption cut'
 foreach($kind in 0..8){Check ([regex]::Matches($fault,("HC074-STARTING-DRIFT kind=$kind sticky=true fence_retained=true")).Count -eq 1) 'Drift case absent'}
 Check ($fault.Contains('HC074-STARTING-CONTROL acknowledged=true ordered=true fsync=true start=false') -and $fault.Contains('test result: ok. 1 passed; 0 failed')) 'Healthy publication missing'
 $raw=Get-Content -LiteralPath (Join-Path $PSScriptRoot ("crash-$lane.log")) -Raw
 $rows=[regex]::Matches($raw,'collector_starting_crash seed=0x074057d1 repeat=([1-3]) stage=(\w+) sigkill=(true|false) fresh_inspection=([12]) empty=false unchanged=true')
 Check ($rows.Count -eq 66) 'Fresh inspection count differs'
 foreach($repeat in 1..3){
  foreach($stage in @($stages+'AfterReturn')){foreach($inspection in 1..2){
   Check (@($rows|Where-Object{$_.Groups[1].Value -eq "$repeat" -and $_.Groups[2].Value -ceq $stage -and $_.Groups[3].Value -ceq 'true' -and $_.Groups[4].Value -eq "$inspection"}).Count -eq 1) 'Missing exact SIGKILL/fresh-inspection row'
  }}
  foreach($inspection in 1..2){Check (@($rows|Where-Object{$_.Groups[1].Value -eq "$repeat" -and $_.Groups[2].Value -ceq 'AfterReturn' -and $_.Groups[3].Value -ceq 'false' -and $_.Groups[4].Value -eq "$inspection"}).Count -eq 1) 'Missing uninterrupted control'}
  Check ([regex]::Matches($raw,("collector_starting_crash_summary seed=0x074057d1 repeat=$repeat sigkills=10 controls=1 fresh_inspections=22 production_enrollment=false start=false")).Count -eq 1) 'Summary differs'
 }
 Check ($raw.Contains('test result: ok. 1 passed; 0 failed')) 'Crash matrix failed'
 foreach($phase in @('fault','crash')){Check ($logs['starting-framed-server-run'].Contains("phase=$phase lane=$lane exit=0 installation=false product=false")) 'Runner gate absent'}
 Check ($logs[('starting-framed-fast-'+$lane)].Contains('10 passed; 0 failed; 2 ignored') -and [regex]::Matches($logs[('starting-framed-fast-'+$lane)],'1 passed; 0 failed').Count -eq 2) 'Fast/API coverage differs'
 Check ($logs[('starting-framed-docs-'+$lane)].Contains('50 passed; 0 failed')) 'Compile-fail count differs'
 Check ($logs[('starting-framed-preconsent-docs-'+$lane)].Contains('6 passed; 0 failed')) 'Preparation API regression differs'
 Check ($logs[('starting-framed-preconsent-fast-'+$lane)].Contains('cases=3072 exclusive=true') -and [regex]::Matches($logs[('starting-framed-preconsent-fast-'+$lane)],'1 passed; 0 failed').Count -eq 2) 'Preparation age/named API regression differs'
 foreach($n in @(24,20,5,7,6)){Check ($logs[('starting-framed-regression-'+$lane)].Contains("$n passed; 0 failed")) 'Regression coverage differs'}
}
Check ([regex]::Matches($logs['starting-framed-server-run'],'preflight=ok supervisor_pid=8839 restarts=0').Count -eq 10) 'Host pre/post count differs'
$post=$logs['starting-framed-postflight']
foreach($s in @('MainPID=8839','NRestarts=0','ExecMainStartTimestamp=Tue 2026-10-06 15:03:40 CEST','ActiveState=active','SubState=running','e81671d6-bf23-4ebf-b8fe-5dad1f45866f','f797629f65385ebfd4d94577a43a2d1d020f71c1e841028dad7d02a51b47ad62','b314953d4deedaeaade3e24468da4844b7f13d348a5d77b8e8f41c672ff44525','postflight=ok no_active_context=true')){Check ($post.Contains($s)) ('Host identity differs: '+$s)}
$run=Get-Content -LiteralPath (Join-Path $PSScriptRoot 'run.sh') -Raw
Check ($m.binaries.Count -eq 2) 'Binary inventory differs'
foreach($b in $m.binaries){Check ($run.Contains($b.sha256+'  '+$b.name)) 'Runner hash differs'}
$repo=(Resolve-Path -LiteralPath (Join-Path $PSScriptRoot '../../../../../..')).Path
Push-Location $repo
try {
 Check ($m.source_blobs.Count -eq 15 -and @($m.source_blobs.path|Select-Object -Unique).Count -eq 15) 'Source inventory differs'
$sourcePaths=@('crates/xtask/tests/performance_contract_074.rs','crates/xtask/tests/support/performance_contract_074_preconsent_frame.rs','crates/xtask/tests/support/performance_contract_074_starting_record.rs','crates/xtask/tests/support/performance_contract_074_starting_publication.rs','docs/COMPAT.md','docs/testing/gated-test-registry.toml','tools/long-run-supervisor-074/src/diagnostic_collector_reservation_storage.rs','tools/long-run-supervisor-074/src/diagnostic_original_start_consent.rs','tools/long-run-supervisor-074/src/diagnostic_validation_frame.rs','tools/long-run-supervisor-074/src/diagnostic_validation_frame_tests.rs','tools/long-run-supervisor-074/src/diagnostic_collector_starting_storage.rs','tools/long-run-supervisor-074/src/diagnostic_collector_starting_publication.rs','tools/long-run-supervisor-074/src/diagnostic_collector_starting_publication_tests.rs','tools/long-run-supervisor-074/src/diagnostic_collector_starting_crash_tests.rs','tools/long-run-supervisor-074/tests/diagnostic_collector_starting_publication.rs')
 Same @($m.source_blobs.path) $sourcePaths 'Source path inventory differs'
 foreach($f in $m.source_blobs){$b=& git rev-parse ($m.source_commit+':'+$f.path);Check ($LASTEXITCODE -eq 0 -and $b -ceq $f.git_blob) 'Source blob differs'}
 & git merge-base --is-ancestor $m.baseline_commit $m.source_commit;Check ($LASTEXITCODE -eq 0) 'Source ancestry differs'
 & git diff --quiet $m.baseline_commit $m.source_commit -- tools/long-run-supervisor-074/src/diagnostic_preconsent_frame.rs tools/long-run-supervisor-074/src/diagnostic_lease.rs tools/long-run-supervisor-074/src/diagnostic_kernel_admission.rs tools/long-run-supervisor-074/src/diagnostic_collector_causal.rs docs/testing/performance/0.74/qualification-manifest.toml
 Check ($LASTEXITCODE -eq 0) 'Consent/deadlines/kernel/challenge/qualification changed'
}finally{Pop-Location}
'Starting publication proof intact: 60 SIGKILL / 6 controls / 132 fresh refusals PASS; runtime lost ack only, production admission CLOSED'
