param([string]$ManifestPath=(Join-Path $PSScriptRoot 'manifest.json'))
$ErrorActionPreference='Stop'
function Check($ok,[string]$why){if(-not $ok){throw $why}}
function Same($a,$b,[string]$why){Check ((@($a|Sort-Object)-join '|') -ceq (@($b|Sort-Object)-join '|')) $why}
$m=Get-Content -LiteralPath $ManifestPath -Raw|ConvertFrom-Json
Check ($m.kind -ceq 'original-collector-pipe-preparation-proof' -and $m.format_version -eq 1 -and $m.baseline_commit -ceq 'a72a1b6a84373f5006504de458599613b01d6327' -and $m.source_commit -match '^[a-f0-9]{40}$') 'Identity differs'
Check ($m.temporary_root -ceq '/tmp/hc074-original-pipes-20261016.eO13aX') 'Root differs'
$yes=@('joined_original_pipe_gate_passed','original_guards_preserved','deadlines_unchanged','private_pipe_backpressure_tested','private_pipe_drop_eof_tested','pipe_fields_drop_before_starting')
$no=@('root_collector_constructed','fd_or_stdio_exported','authenticated_product_start','all_writers_revoked','completion_or_later_cells','new_sigkill_proof','production_trust_enrolled','signed_production_custody','installed_release_changed','supervisor_stopped','clock_changed','product_workload_run','qualification_changed','performance_claim','server_ready')
Same @($m.scope.PSObject.Properties.Name) @($yes+$no) 'Scope inventory differs'
foreach($k in $yes){Check ($m.scope.$k -is [bool] -and $m.scope.$k) ('Missing result: '+$k)}
foreach($k in $no){Check ($m.scope.$k -is [bool] -and -not $m.scope.$k) ('Unsupported claim: '+$k)}
Same $m.coverage.compilers @('1.94.0','1.88.0') 'Compilers differ'
$expected='{"joined_cases_per_compiler":19,"construction_interruptions_per_compiler":6,"descriptor_drifts_per_compiler":8,"live_drifts_per_compiler":2,"acknowledgement_drift_per_compiler":1,"pre_refused_constructor_per_compiler":1,"healthy_drop_control_per_compiler":1,"kernel_units_per_compiler":4,"public_api_per_compiler":1,"randomized_flags_per_compiler":4096,"flag_edges_per_compiler":6,"selected_compile_fail_per_compiler":57,"regressions_per_compiler":63,"static_contracts":187,"registered_gates":180}'|ConvertFrom-Json
foreach($p in $expected.PSObject.Properties){Check ($m.coverage.($p.Name) -eq $p.Value) ('Coverage differs: '+$p.Name)}
Check ($m.artifacts.Count -eq $m.commands.Count+3 -and @($m.artifacts.path|Select-Object -Unique).Count -eq $m.artifacts.Count) 'Duplicate/missing artifacts'
$requiredKeys=@('pipes-red','pipes-baseline','pipes-fast-first','pipes-final-fast-194','pipes-final-fast-188','pipes-final-dev-win194','pipes-final-dev-win188','pipes-final-clippy-linux194','pipes-final-clippy-linux188','pipes-final-static','pipes-scoped-fmt','pipes-fmt','pipes-registry','pipes-doccheck','pipes-localcontract','pipes-release-194','pipes-release-188','pipes-server-root-create','pipes-binary-hashes','pipes-transfer','pipes-prior-proof-collector-starting-framed-20261016','pipes-prior-proof-preconsent-frame-20261016','pipes-final-check-linux194','pipes-final-check-linux188','pipes-final-docs-linux194','pipes-final-docs-linux188','pipes-regression-194','pipes-regression-188','pipes-ship-guard','pipes-server-run','pipes-postflight','pipes-postflight-assembly')
Same @($m.commands.key) $requiredKeys 'Command inventory differs'
Check ($m.commands.Count -eq 32) 'Duplicate command'
Same @($m.artifacts.path) @(@('run.sh','matrix-194.log','matrix-188.log')+@($m.commands|ForEach-Object{'logs/'+$_.key+'.json'})) 'Artifact inventory differs'
foreach($a in $m.artifacts){
 Check ($a.path -notmatch '(^/|\.\.|\\|:)') 'Unsafe artifact path'
 $p=Join-Path $PSScriptRoot $a.path
 Check ((Get-Item -LiteralPath $p).Length -eq $a.bytes -and (Get-FileHash -LiteralPath $p -Algorithm SHA256).Hash.ToLowerInvariant() -ceq $a.sha256) ('Artifact differs: '+$a.path)
}
$logs=@{}
foreach($cmd in $m.commands){
 $r=Get-Content -LiteralPath (Join-Path $PSScriptRoot ('logs/'+$cmd.key+'.json')) -Raw|ConvertFrom-Json
 $expectedExit=if($cmd.key -in @('pipes-red','pipes-ship-guard')){1}else{0}
 Check ($r.key -ceq $cmd.key -and $r.exit_code -eq $expectedExit -and $cmd.exit_code -eq $expectedExit -and -not [string]::IsNullOrWhiteSpace($r.command)) ('Command differs: '+$cmd.key)
 $logs[$cmd.key]=$r.output
}
Check ($logs['pipes-red'].Contains('E0599') -and $logs['pipes-ship-guard'].Contains('ship admission is closed')) 'Red guards absent'
Check ($logs['pipes-postflight-assembly'].Contains('preflight: command not found')) 'Postflight assembly failure lost'
Check (-not $logs['pipes-postflight'].Contains('command not found')) 'Invalid postflight promoted'
$cuts=@('BeforeCreate','StdoutCreated','StdoutReady','StderrCreated','StderrReady','Acknowledging')
foreach($lane in @('194','188')){
 $raw=Get-Content -LiteralPath (Join-Path $PSScriptRoot ("matrix-$lane.log")) -Raw
 $rows=[regex]::Matches($raw,'HC074-PIPE-INTERRUPT cut=(\w+) held=true refused=true unchanged=true')
 Check ($rows.Count -eq 6) 'Interruption count differs'
 Same @($rows|ForEach-Object{$_.Groups[1].Value}) $cuts 'Construction cuts differ'
 foreach($kind in 0..7){Check ([regex]::Matches($raw,("HC074-PIPE-DRIFT kind=$kind held=true refused=true")).Count -eq 1) 'Descriptor case missing'}
 foreach($concurrent in @('false','true')){Check ([regex]::Matches($raw,("HC074-PIPE-LIVE concurrent=$concurrent held=true refused=true")).Count -eq 1) 'Live case missing'}
 foreach($s in @('HC074-PIPE-ACK-DRIFT closing=true held=true refused=true','HC074-PIPE-PRE-REFUSED allocated=0 held=true refused=true','HC074-PIPE-CONTROL original=true private=true drop_eof=true execution=false','test result: ok. 1 passed; 0 failed')){Check ($raw.Contains($s)) ('Control missing: '+$s)}
 Check ($logs['pipes-server-run'].Contains("lane=$lane exit=0 output=false execution=false")) 'Server result missing'
 Check ($logs[('pipes-final-fast-'+$lane)].Contains('4 passed; 0 failed') -and $logs[('pipes-final-fast-'+$lane)].Contains('1 passed; 0 failed') -and $logs[('pipes-final-fast-'+$lane)].Contains('randomized=4096 edges=6 exact=true')) 'Fast/API/seed proof differs'
 Check ($logs[('pipes-final-docs-linux'+$lane)].Contains('57 passed; 0 failed')) 'Compile-fail coverage differs'
 foreach($n in @(1,24,20,5,7,6)){Check ($logs[('pipes-regression-'+$lane)].Contains("$n passed; 0 failed")) 'Regression coverage differs'}
}
Check ($logs['pipes-final-static'].Contains('187 passed; 0 failed') -and $logs['pipes-registry'].Contains('180 discovered gates, all registered')) 'Static registry differs'
Check ([regex]::Matches($logs['pipes-server-run'],'preflight=ok supervisor_pid=8839 restarts=0').Count -eq 5) 'Host pre/post count differs'
foreach($s in @('MainPID=8839','NRestarts=0','ExecMainStartTimestamp=Tue 2026-10-06 15:03:40 CEST','ActiveState=active','SubState=running','e81671d6-bf23-4ebf-b8fe-5dad1f45866f','f797629f65385ebfd4d94577a43a2d1d020f71c1e841028dad7d02a51b47ad62','b314953d4deedaeaade3e24468da4844b7f13d348a5d77b8e8f41c672ff44525','postflight=ok no_active_context=true')){Check ($logs['pipes-postflight'].Contains($s)) ('Host differs: '+$s)}
$run=Get-Content -LiteralPath (Join-Path $PSScriptRoot 'run.sh') -Raw
Check ($m.binaries.Count -eq 2) 'Binary inventory differs'
foreach($b in $m.binaries){Check ($run.Contains($b.sha256+'  '+$b.name)) 'Runner hash differs'}
$repo=(Resolve-Path -LiteralPath (Join-Path $PSScriptRoot '../../../../../..')).Path
Push-Location $repo
try{
 Check ($m.source_blobs.Count -eq 10 -and @($m.source_blobs.path|Select-Object -Unique).Count -eq 10) 'Source inventory differs'
 Same @($m.source_blobs.path) @('crates/xtask/tests/performance_contract_074.rs','crates/xtask/tests/support/performance_contract_074_starting_publication.rs','crates/xtask/tests/support/performance_contract_074_original_pipes.rs','tools/long-run-supervisor-074/src/diagnostic_collector_starting_publication.rs','tools/long-run-supervisor-074/src/diagnostic_collector_starting_publication_tests.rs','tools/long-run-supervisor-074/src/diagnostic_original_collector_pipes.rs','tools/long-run-supervisor-074/src/diagnostic_original_collector_pipe_kernel_tests.rs','tools/long-run-supervisor-074/src/diagnostic_original_collector_pipes_tests.rs','tools/long-run-supervisor-074/tests/diagnostic_original_collector_pipes.rs','docs/testing/gated-test-registry.toml') 'Source paths differ'
 foreach($f in $m.source_blobs){$b=& git rev-parse ($m.source_commit+':'+$f.path);Check ($LASTEXITCODE -eq 0 -and $b -ceq $f.git_blob) 'Build/commit source differs'}
 & git diff --quiet $m.baseline_commit $m.source_commit -- tools/long-run-supervisor-074/src/diagnostic_lease.rs tools/long-run-supervisor-074/src/diagnostic_kernel_admission.rs tools/long-run-supervisor-074/src/diagnostic_collector_starting_storage.rs tools/long-run-supervisor-074/src/diagnostic_preconsent_frame.rs docs/testing/performance/0.74/qualification-manifest.toml docs/COMPAT.md
 Check ($LASTEXITCODE -eq 0) 'Frozen runtime/qualification changed'
}finally{Pop-Location}
'Original pipe proof intact: 19 joined cases/compiler PASS; private kernel pipes only, original start and server admission CLOSED'
