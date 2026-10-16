param([string]$ManifestPath = (Join-Path $PSScriptRoot 'manifest.json'))
$ErrorActionPreference = 'Stop'
function Check($ok, [string]$message) { if (-not $ok) { throw $message } }
$m = Get-Content -LiteralPath $ManifestPath -Raw | ConvertFrom-Json
Check ($m.format_version -eq 1 -and $m.kind -ceq 'approved-local-wsl-clock-control-refused') 'Wrong kind'
Check ($m.preregistration_commit -ceq '32b5f2ba6f6c2c2a6275f9615a5117f51ab57674') 'Wrong preregistration'
Check ($m.original_source_snapshot -ceq '../validation-frame-20261016/manifest.json') 'Wrong original snapshot'
$trueFields = @('service_temporarily_stopped','service_restored')
$falseFields = @('preflight_passed','runtime_changed','deadline_changed','clock_value_changed_by_controller','windows_service_changed','clocksource_changed','server_mutated','product_workload_run','qualification_changed','candidate_committed','server_ready')
$zeroFields = @('matrices_started','budget_cohorts_started')
$expected = @($trueFields + $falseFields + $zeroFields | Sort-Object) -join '|'
Check ((@($m.scope.PSObject.Properties.Name | Sort-Object) -join '|') -ceq $expected) 'Scope inventory differs'
foreach ($key in $trueFields) { Check ($m.scope.$key -is [bool] -and $m.scope.$key) ('Missing restoration fact: ' + $key) }
foreach ($key in $falseFields) { Check ($m.scope.$key -is [bool] -and -not $m.scope.$key) ('Unsupported claim: ' + $key) }
foreach ($key in $zeroFields) { Check ($m.scope.$key -eq 0 -and $m.scope.$key -isnot [bool]) ('Unrun cohort claimed: ' + $key) }
$known = @{
    'run.sh' = '7e8b84b9bc39afc2fc92e0a187701b0557870811c7d4cc70ef99fccbca5455b7'
    'run.json' = '9013c0826b61a3acdf5ebb967caa87342c54fd8a643b5b953c5e2bc0dbf99755'
    'restoration.json' = 'ef260c435fe83001a43ea9d8880c259f89f4b22cefa6a59fa7f5f73beba13fa1'
}
Check ($m.artifacts.Count -eq 3 -and (@($m.artifacts.path | Sort-Object) -join '|') -ceq (@($known.Keys | Sort-Object) -join '|')) 'Artifact inventory differs'
foreach ($a in $m.artifacts) {
    $path = Join-Path $PSScriptRoot $a.path
    Check ($a.sha256 -ceq $known[$a.path] -and (Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash.ToLowerInvariant() -ceq $a.sha256 -and (Get-Item -LiteralPath $path).Length -eq $a.bytes) ('Artifact differs: ' + $a.path)
}
$r = Get-Content -LiteralPath (Join-Path $PSScriptRoot 'run.json') -Raw | ConvertFrom-Json
$v = Get-Content -LiteralPath (Join-Path $PSScriptRoot 'restoration.json') -Raw | ConvertFrom-Json
Check ($r.exit_code -eq 1 -and $v.exit_code -eq 0) 'Outcomes differ'
Check ($m.outcomes.wrapper_reported_result -eq 93 -and $m.outcomes.outer_run_exit_code -eq 1 -and $m.outcomes.independent_restoration_exit_code -eq 0) 'Reported outcomes differ'
Check ($r.output.Contains('service_before=active') -and $r.output.Contains('service_during=inactive') -and $r.output.Contains('service_restored=active result=93')) 'Service transition missing'
Check ($v.output.StartsWith('active') -and $v.output.Contains('UnitFileState=enabled')) 'Independent restoration missing'
Check (-not $r.output.Contains('matrix_begin=') -and -not $r.output.Contains('budget_begin=')) 'Unexpected test start'
Check ($m.preflight.planned_seconds -eq 90 -and $m.preflight.guard_delta_ns -eq 100000000) 'Preflight guard differs'
Check (($m.preflight.observed_indices -join ',') -ceq '0,1,2,3,4,5,6' -and $m.preflight.refusal_index -eq 7 -and $m.preflight.realtime_delta_ns -eq -3151529293 -and $m.preflight.boottime_delta_ns -eq 1000220159) 'Preflight result differs'
$rows = [regex]::Matches($r.output, 'HC074-CLOCK-PREFLIGHT index=(\d+) realtime_ns=(\d+) boottime_ns=(\d+)')
Check ($rows.Count -eq 7) 'Sample count differs'
for ($i=0; $i -lt 7; $i++) { Check ($rows[$i].Groups[1].Value -eq $i) 'Sample order differs' }
Check ($r.output.Contains('HC074-CLOCK-PREFLIGHT refused index=7 realtime_delta=-3151529293 boottime_delta=1000220159')) 'Missing refusal'
& powershell -NoProfile -File (Join-Path $PSScriptRoot '../validation-frame-20261016/verify.ps1')
Check ($LASTEXITCODE -eq 0) 'Original source packet differs'
'WSL clock control evidence: intact; preflight REFUSED, service RESTORED, no test cohort started'
