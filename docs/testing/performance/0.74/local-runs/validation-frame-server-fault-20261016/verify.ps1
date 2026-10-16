param([string]$ManifestPath = (Join-Path $PSScriptRoot 'manifest.json'))
$ErrorActionPreference = 'Stop'
function Check($ok, [string]$message) { if (-not $ok) { throw $message } }
function Same($a, $b, [string]$message) { Check ((@($a | Sort-Object) -join '|') -ceq (@($b | Sort-Object) -join '|')) $message }
$repo = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot '../../../../../..')).Path
$m = Get-Content -LiteralPath $ManifestPath -Raw | ConvertFrom-Json
Check ($m.format_version -eq 1 -and $m.kind -ceq 'isolated-server-validation-frame-fault') 'Packet kind differs'
Check ($m.source_commit -ceq '7b7992701b9d5b69f8d8fca27aa9e016b8c80f2c' -and $m.source_tree -ceq 'daf94165fa9169bd1a28ce6ec634c644c9db84b2' -and $m.preregistration_commit -ceq '51dccae3') 'Source identity differs'
Check ($m.temporary_root -ceq '/tmp/hc074-validation-frame-20261016.htNCLN') 'Temporary root differs'
$trueFields = @('fault_matrix_complete','isolated_temporary_writes','supervisor_identity_unchanged')
$falseFields = @('installed_release_changed','supervisor_stopped','clock_changed','budget_cohort_run','durable_starting_written','new_sigkill_matrix_run','authenticated_process_started','all_writers_revoked','production_enrollment','product_workload_run','qualification_changed','server_ready')
Same @($m.scope.PSObject.Properties.Name) @($trueFields + $falseFields) 'Scope inventory differs'
foreach ($key in $trueFields) { Check ($m.scope.$key -is [bool] -and $m.scope.$key) ('Missing result: ' + $key) }
foreach ($key in $falseFields) { Check ($m.scope.$key -is [bool] -and -not $m.scope.$key) ('Unsupported claim: ' + $key) }
Check ($m.host.alias -ceq 'hydracache-ax42' -and $m.host.test_uid -eq 1000 -and $m.host.test_gid -eq 1000 -and $m.host.supervisor_pid -eq 8839 -and $m.host.supervisor_restarts -eq 0 -and $m.host.boot_id -ceq 'e81671d6-bf23-4ebf-b8fe-5dad1f45866f') 'Host observations differ'
Check ($m.host.installed_source -ceq '543108f1ccd206ae670803c2617f07e7fa91ae62' -and $m.host.installed_binary_sha256 -ceq 'f797629f65385ebfd4d94577a43a2d1d020f71c1e841028dad7d02a51b47ad62' -and $m.host.installed_receipt_sha256 -ceq 'b314953d4deedaeaade3e24468da4844b7f13d348a5d77b8e8f41c672ff44525') 'Installed observations differ'
$expectedPaths = @('linux-check.json','linux-clippy.json','matrix-188.log','matrix-194.log','msrv-linux-clippy.json','msrv-windows-check.json','postflight.json','run.json','run.sh','static.json','windows-check.json','windows-clippy.json')
Same @($m.artifacts.path) $expectedPaths 'Artifact inventory differs'
Check ($m.artifacts.Count -eq 12) 'Duplicate artifacts'
foreach ($a in $m.artifacts) {
    $path = Join-Path $PSScriptRoot $a.path
    Check ((Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash.ToLowerInvariant() -ceq $a.sha256 -and (Get-Item -LiteralPath $path).Length -eq $a.bytes) ('Artifact differs: ' + $a.path)
    if ($a.path.EndsWith('.json')) {
        $c = Get-Content -LiteralPath $path -Raw | ConvertFrom-Json
        Check ($c.format_version -eq 1 -and $c.exit_code -eq 0 -and -not [string]::IsNullOrWhiteSpace($c.command)) ('Capture failed: ' + $a.path)
    }
}
$oldPath = Join-Path $PSScriptRoot '../validation-frame-20261016/manifest.json'
Check ($m.original_candidate_manifest -ceq '../validation-frame-20261016/manifest.json') 'Original packet differs'
$old = Get-Content -LiteralPath $oldPath -Raw | ConvertFrom-Json
Push-Location $repo
try {
    $tree = & git rev-parse ($m.source_commit + '^{tree}')
    Check ($LASTEXITCODE -eq 0 -and $tree -ceq $m.source_tree) 'Git tree differs'
    & git merge-base --is-ancestor $m.preregistration_commit $m.source_commit
    Check ($LASTEXITCODE -eq 0) 'Preregistration ancestry differs'
    $oldRelative = 'docs/testing/performance/0.74/local-runs/validation-frame-20261016/manifest.json'
    $oldBlob = & git rev-parse ($m.source_commit + ':' + $oldRelative)
    $workingBlob = & git hash-object -- $oldRelative
    Check ($LASTEXITCODE -eq 0 -and $oldBlob -ceq $workingBlob) 'Historical manifest changed'
    foreach ($f in @($old.selected_sources) + @($old.frozen_inputs)) {
        $blob = & git rev-parse ($m.source_commit + ':' + $f.path)
        Check ($LASTEXITCODE -eq 0 -and $blob -ceq $f.git_blob) ('Original source differs: ' + $f.path)
    }
    $delta = @(& git diff --name-only ($m.source_commit + '^') $m.source_commit)
    Check ($LASTEXITCODE -eq 0) 'Cannot inspect source delta'
    Same $delta @($old.selected_sources.path) 'Frame source scope differs'
} finally { Pop-Location }
Check ($m.lanes.Count -eq 2) 'Lane count differs'
for ($i=0; $i -lt 2; $i++) {
    $lane = $m.lanes[$i]
    $compiler = @('1.94.0','1.88.0')[$i]
    $logName = @('matrix-194.log','matrix-188.log')[$i]
    $sha = @('65f901ca510c240e4beff482c19d58ac6279ab685ee592c072142f98e4a91590','c2e6ff7a751a7b6c0278f5d37910dba2c8128ea7356793a9af63373a7f013187')[$i]
    Check ($lane.compiler -ceq $compiler -and $lane.log -ceq $logName -and $lane.binary_sha256 -ceq $sha -and $lane.exit_code -eq 0 -and $lane.mutation_pairs -eq 40 -and $lane.reference_cases -eq 5 -and $lane.control_cases -eq 3) 'Lane result differs'
    $raw = Get-Content -LiteralPath (Join-Path $PSScriptRoot $logName) -Raw
    $rows = [regex]::Matches($raw, 'HC074-FRAME-DRIFT cut=(\d) kind=(\d) refused=true fence_retained=true')
    Check ($rows.Count -eq 40) 'Mutation coverage differs'
    for ($j=0; $j -lt 40; $j++) { Check ($rows[$j].Groups[1].Value -eq [math]::Floor($j/5) -and $rows[$j].Groups[2].Value -eq ($j%5)) 'Mutation pair order differs' }
    $rows = [regex]::Matches($raw, 'HC074-FRAME-REFERENCE kind=(\d) refused=true')
    Check ($rows.Count -eq 5) 'Reference coverage differs'
    for ($j=0; $j -lt 5; $j++) { Check ($rows[$j].Groups[1].Value -eq $j) 'Reference order differs' }
    foreach ($marker in @('HC074-FRAME-CONTROL healthy_before_fault=true ordered=true','HC074-FRAME-CONSTRUCTION refused=true retained=true','HC074-FRAME-CONCURRENT refused=true original_owners=true','1 passed; 0 failed; 0 ignored','993 filtered out')) { Check ($raw.Contains($marker)) ('Missing control: ' + $marker) }
    Check ($raw.Contains(('finished in ' + $lane.elapsed_seconds.ToString('F2',[Globalization.CultureInfo]::InvariantCulture) + 's'))) 'Test duration differs'
    Check (-not $raw.Contains('HC074-FRAME-BUDGET') -and -not $raw.Contains('panicked at')) 'Wrong or failing workload'
}
$run = Get-Content -LiteralPath (Join-Path $PSScriptRoot 'run.json') -Raw | ConvertFrom-Json
$post = Get-Content -LiteralPath (Join-Path $PSScriptRoot 'postflight.json') -Raw | ConvertFrom-Json
Check ($run.output.Contains('lane=194 exit=0') -and $run.output.Contains('lane=188 exit=0') -and $run.output.Contains('complete=true failed=0 supervisor_unchanged=true')) 'Wrapper completion differs'
foreach ($marker in @('MainPID=8839','NRestarts=0','ActiveState=active','SubState=running',$m.host.boot_id,$m.host.installed_binary_sha256,$m.host.installed_receipt_sha256,'active-campaign=absent')) { Check ($post.output.Contains($marker)) ('Postflight differs: ' + $marker) }
$static = Get-Content -LiteralPath (Join-Path $PSScriptRoot 'static.json') -Raw | ConvertFrom-Json
Check ($static.output.Contains('183 passed; 0 failed')) 'Frame static gate differs'
'Server validation frame evidence: intact; 48 cases per compiler PASS; production and server lifecycle admission CLOSED'
