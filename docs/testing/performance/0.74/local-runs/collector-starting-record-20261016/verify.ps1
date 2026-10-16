param([string]$ManifestPath = (Join-Path $PSScriptRoot 'manifest.json'), [switch]$CheckWorkingCopy)
$ErrorActionPreference = 'Stop'
function Check($ok,[string]$message) { if (-not $ok) { throw $message } }
function Same($a,$b,[string]$message) { Check ((@($a | Sort-Object)-join '|') -ceq (@($b | Sort-Object)-join '|')) $message }
$repo=(Resolve-Path -LiteralPath (Join-Path $PSScriptRoot '../../../../../..')).Path
$m=Get-Content -LiteralPath $ManifestPath -Raw | ConvertFrom-Json
Check ($m.format_version -eq 1 -and $m.kind -ceq 'collector-starting-byte-record-only') 'Packet kind differs'
Check ($m.source_commit -ceq '8d1b5e2a6921f23ba0ba1d03af7e863f1aa1be25' -and $m.source_tree -ceq '426f69d8d26020e65da145e7966e806cf6ce7d6d' -and $m.parent_commit -ceq '15beb95a' -and $m.baseline_commit -ceq '7b7992701b9d5b69f8d8fca27aa9e016b8c80f2c' -and $m.preregistration_commit -ceq 'f04931a77c27c0aac2901c860de3dcc3b2b95985') 'Source identity differs'
$trueFields=@('byte_record_implemented','original_storage_inventory_unchanged')
$falseFields=@('actual_starting_written','runtime_starting_acknowledged','original_runtime_owner_recovered','signatures_verified_by_byte_inspector','new_sigkill_matrix_run','server_operation_in_this_packet','product_workload_run','deadline_changed','qualification_changed','performance_claim','server_ready','full_release_verify_run')
Same @($m.scope.PSObject.Properties.Name) @($trueFields+$falseFields) 'Scope inventory differs'
foreach($key in $trueFields) { Check ($m.scope.$key -is [bool] -and $m.scope.$key) ('Missing result: '+$key) }
foreach($key in $falseFields) { Check ($m.scope.$key -is [bool] -and -not $m.scope.$key) ('Unsupported claim: '+$key) }
Check ($m.schema.version -ceq 'diagnostic-collector-starting-074-v1' -and $m.schema.fixed_future_name -ceq 'diagnostic-collector-starting-074.json' -and $m.schema.max_bytes -eq 16384 -and $m.schema.signed_consent_max_bytes -eq 8192) 'Schema differs'
$f=$m.fixture
Check ($f.golden_bytes -eq 3038 -and $f.strict_refused_prefixes -eq 3038 -and $f.mutation_seed -ceq '0x074057a1' -and $f.leaf_mutations -eq 58 -and $f.rehashed_binding_mutations -eq 9 -and $f.rehashed_grammar_state_mutations -eq 7 -and $f.unit_tests_per_compiler -eq 5 -and $f.public_api_tests_per_compiler -eq 1 -and $f.compile_fail_examples_per_compiler -eq 2 -and $f.unchanged_regression_tests_per_compiler -eq 62 -and $f.static_contracts -eq 184 -and $f.discovered_registered_gates -eq 177) 'Fixture coverage differs'
$selected=@('crates/xtask/tests/performance_contract_074.rs','crates/xtask/tests/support/performance_contract_074_consent_budget.rs','crates/xtask/tests/support/performance_contract_074_original_start_consent.rs','crates/xtask/tests/support/performance_contract_074_validation_frame.rs','crates/xtask/tests/support/performance_contract_074_starting_record.rs','docs/COMPAT.md','docs/testing/gated-test-registry.toml','tools/long-run-supervisor-074/src/diagnostic_collector_reserved.rs','tools/long-run-supervisor-074/src/diagnostic_collector_starting_record.rs','tools/long-run-supervisor-074/src/diagnostic_collector_starting_record_tests.rs','tools/long-run-supervisor-074/tests/diagnostic_collector_starting_record.rs')
$frozen=@('Cargo.toml','Cargo.lock','tools/long-run-supervisor-074/Cargo.toml','tools/long-run-supervisor-074/src/lib.rs','tools/long-run-supervisor-074/src/diagnostic_collector_reservation_storage.rs','tools/long-run-supervisor-074/src/diagnostic_controller_request_attempt.rs','tools/long-run-supervisor-074/src/diagnostic_collector_causal.rs','tools/long-run-supervisor-074/src/diagnostic_collector_progress.rs','tools/long-run-supervisor-074/src/diagnostic_original_start_consent.rs','tools/long-run-supervisor-074/src/diagnostic_validation_frame.rs','tools/long-run-supervisor-074/src/diagnostic_validation_frame_tests.rs','tools/long-run-supervisor-074/src/diagnostic_validation_frame_budget_tests.rs','tools/long-run-supervisor-074/src/diagnostic_start_consent.rs','tools/long-run-supervisor-074/src/diagnostic_kernel_admission.rs','tools/long-run-supervisor-074/src/diagnostic_kernel_start_consent.rs','tools/long-run-supervisor-074/src/diagnostic_worker_start_consent.rs','tools/long-run-supervisor-074/src/diagnostic_ipc.rs','tools/long-run-supervisor-074/src/diagnostic_lease.rs','tools/long-run-supervisor-074/src/diagnostic_transition.rs','tools/long-run-supervisor-074/src/host_execution.rs','tools/long-run-supervisor-074/src/server.rs','docs/testing/performance/0.74/qualification-manifest.toml')
Same @($m.selected_sources.path) $selected 'Selected source inventory differs'
Same @($m.frozen_runtime_inputs.path) $frozen 'Frozen runtime inventory differs'
Check ($m.selected_sources.Count -eq 11 -and $m.frozen_runtime_inputs.Count -eq 22) 'Duplicate source'
Push-Location $repo
try {
    $tree=& git rev-parse ($m.source_commit+'^{tree}')
    Check ($LASTEXITCODE -eq 0 -and $tree -ceq $m.source_tree) 'Git tree differs'
    $parent=& git rev-parse ($m.source_commit+'^')
    $expectedParent=& git rev-parse $m.parent_commit
    Check ($LASTEXITCODE -eq 0 -and $parent -ceq $expectedParent) 'Git parent differs'
    & git merge-base --is-ancestor $m.preregistration_commit $m.source_commit
    Check ($LASTEXITCODE -eq 0) 'Preregistration ancestry differs'
    $delta=@(& git diff --name-only $parent $m.source_commit)
    Check ($LASTEXITCODE -eq 0) 'Cannot inspect delta'
    Same $delta $selected 'Source scope differs'
    foreach($file in @($m.selected_sources)+@($m.frozen_runtime_inputs)) {
        $blob=& git rev-parse ($m.source_commit+':'+$file.path)
        Check ($LASTEXITCODE -eq 0 -and $blob -ceq $file.git_blob) ('Source differs: '+$file.path)
        if ($frozen -ccontains $file.path) {
            $before=& git rev-parse ($m.baseline_commit+':'+$file.path)
            Check ($LASTEXITCODE -eq 0 -and $before -ceq $blob) ('Runtime changed: '+$file.path)
        } elseif ($CheckWorkingCopy) {
            Check ((Get-FileHash -LiteralPath $file.path -Algorithm SHA256).Hash.ToLowerInvariant() -ceq $file.checkout_sha256) ('Working bytes differ: '+$file.path)
        }
    }
} finally { Pop-Location }
$keys=@('starting-record-red-api','starting-record-first-unit','starting-record-unit-fixed','starting-record-unit-194','starting-record-unit-188','starting-record-static-focused','starting-record-static-all','starting-record-static-reference-failure','starting-record-static-final','starting-record-win-check','starting-record-linux-clippy','starting-record-linux-msrv-clippy','starting-record-win-clippy','starting-record-msrv-win-clippy','starting-record-docs-194','starting-record-docs-188','starting-record-api-194','starting-record-api-188','starting-record-regression-194','starting-record-regression-188','starting-record-xtask-clippy-194','starting-record-xtask-clippy-188','starting-record-fmt','starting-record-registry','starting-record-doc-check','starting-record-local-contract','starting-record-static-after-registry','starting-record-fmt-final','starting-record-registry-final','starting-record-post-format-check-194','starting-record-post-format-check-188','starting-record-ship-guard','starting-record-final-tests-194','starting-record-final-tests-188','starting-record-final-linux-clippy-194','starting-record-final-linux-clippy-188','starting-record-final-win-clippy-194','starting-record-final-win-clippy-188')
$failed=@('starting-record-red-api','starting-record-first-unit','starting-record-static-all','starting-record-static-reference-failure','starting-record-fmt','starting-record-registry','starting-record-ship-guard')
Same @($m.commands.key) $keys 'Command inventory differs'
Same @($m.artifacts.path) @($keys | ForEach-Object { 'logs/'+$_+'.json' }) 'Artifact inventory differs'
Check ($m.commands.Count -eq 38 -and $m.artifacts.Count -eq 38) 'Duplicate command/artifact'
$logs=@{}
foreach($cmd in $m.commands) {
    $name='logs/'+$cmd.key+'.json'
    $a=@($m.artifacts | Where-Object path -CEQ $name)
    Check ($a.Count -eq 1) ('Artifact missing: '+$name)
    $path=Join-Path $PSScriptRoot $name
    Check ((Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash.ToLowerInvariant() -ceq $a[0].sha256 -and (Get-Item -LiteralPath $path).Length -eq $a[0].bytes) ('Artifact differs: '+$name)
    $c=Get-Content -LiteralPath $path -Raw | ConvertFrom-Json
    $expected=if($failed -ccontains $cmd.key){1}else{0}
    Check ($c.format_version -eq 1 -and $c.key -ceq $cmd.key -and $c.exit_code -eq $expected -and $cmd.exit_code -eq $expected -and -not [string]::IsNullOrWhiteSpace($c.command)) ('Outcome differs: '+$cmd.key)
    $logs[$cmd.key]=$c.output
}
Check ($logs['starting-record-red-api'].Contains('E0432') -and $logs['starting-record-red-api'].Contains('starting_record')) 'Missing API red'
foreach($lane in @('194','188')) {
    $raw=$logs[('starting-record-final-tests-'+$lane)]
    foreach($s in @('5 passed; 0 failed','1 passed; 0 failed','mutation_seed=0x074057a1 leaves=58','strict_prefixes=3038','rehashed_binding_mutations=9','rehashed_grammar_state_mutations=7','signature_shape_only=true')) { Check ($raw.Contains($s)) ('Missing final coverage: '+$s) }
    Check ($logs[('starting-record-docs-'+$lane)].Contains('2 passed; 0 failed')) 'Compile-fail coverage differs'
    foreach($n in @(24,20,5,7,6)) { Check ($logs[('starting-record-regression-'+$lane)].Contains(($n.ToString()+' passed; 0 failed'))) 'Regression count differs' }
}
Check ($logs['starting-record-static-after-registry'].Contains('184 passed; 0 failed')) 'Final static gate differs'
Check ($logs['starting-record-registry-final'].Contains('177 discovered gates, all registered')) 'Final registry differs'
Check ($logs['starting-record-ship-guard'].Contains('ship admission is closed')) 'Ship guard missing'
'Collector Starting byte evidence: intact; byte-only PASS, actual publication/process/server admission CLOSED'
