param([string]$ManifestPath=(Join-Path $PSScriptRoot 'manifest.json'))
$ErrorActionPreference='Stop'
function Check($ok,[string]$why) { if(-not $ok){throw $why} }
function Same($a,$b,[string]$why) { Check ((@($a|Sort-Object)-join '|') -ceq (@($b|Sort-Object)-join '|')) $why }
$m=Get-Content -LiteralPath $ManifestPath -Raw|ConvertFrom-Json
Check ($m.format_version -eq 1 -and $m.kind -ceq 'original-preconsent-constant-depth-semantic-proof') 'Identity differs'
Check ($m.source_commit -ceq '77b3ad15c8f56cd1ef41c02e31c1bad622126a55' -and $m.baseline_commit -ceq 'ce4a0f54efd3421129c7239f54e7adab32288b0d' -and $m.temporary_root -ceq '/tmp/hc074-preconsent-20261016.m9VZny') 'Source/root differs'
$yes=@('consent_preparation_semantics_passed','original_guards_preserved','deadlines_unchanged')
$no=@('publication_tested','sigkill_tested','signature_cache','installed_release_changed','supervisor_stopped','product_workload_run','qualification_changed','performance_claim','server_ready')
Same @($m.scope.PSObject.Properties.Name) @($yes+$no) 'Scope inventory differs'
foreach($k in $yes){Check ($m.scope.$k -is [bool] -and $m.scope.$k) ('Missing result: '+$k)}
foreach($k in $no){Check ($m.scope.$k -is [bool] -and -not $m.scope.$k) ('Unsupported claim: '+$k)}
Same $m.coverage.compilers @('1.94.0','1.88.0') 'Compilers differ'
$expected='{"drift_cases_per_compiler":35,"old_reference_refusals_per_compiler":5,"consent_refusals_per_compiler":3,"invalid_ids_per_compiler":2,"pre_refused_constructor_per_compiler":1,"healthy_per_compiler":1,"closing_race_per_compiler":1,"semantic_cases_per_compiler":48,"age_edges_per_compiler":3072,"regressions_per_compiler":62,"compile_fail_per_compiler":6,"public_api_per_compiler":1,"static_contracts":185,"registered_gates":178}'|ConvertFrom-Json
foreach($p in $expected.PSObject.Properties){Check ($m.coverage.($p.Name) -eq $p.Value) ('Coverage differs: '+$p.Name)}
Check ($m.artifacts.Count -eq 34 -and @($m.artifacts.path|Select-Object -Unique).Count -eq 34) 'Artifact inventory differs'
foreach($a in $m.artifacts){
 Check ($a.path -notmatch '(^/|\.\.|\\|:)') 'Unsafe artifact path'
 $p=Join-Path $PSScriptRoot $a.path
 Check ((Get-Item -LiteralPath $p).Length -eq $a.bytes -and (Get-FileHash -LiteralPath $p -Algorithm SHA256).Hash.ToLowerInvariant() -ceq $a.sha256) ('Artifact differs: '+$a.path)
}
function Log([string]$k){(Get-Content -LiteralPath (Join-Path $PSScriptRoot ('logs/'+$k+'.json')) -Raw|ConvertFrom-Json)}
foreach($a in $m.artifacts|Where-Object path -like 'logs/*.json'){
 $r=Get-Content -LiteralPath (Join-Path $PSScriptRoot $a.path) -Raw|ConvertFrom-Json
 $exit=if($r.key -in @('preconsent-red-api','preconsent-static-first','preconsent-static-first-failure','preconsent-launch-before-transfer')){1}else{0}
 Check ($r.exit_code -eq $exit -and -not [string]::IsNullOrWhiteSpace($r.command)) ('Command outcome differs: '+$a.path)
}
Check ((Log 'preconsent-red-api').output.Contains('E0599') -and (Log 'preconsent-static-first-failure').output.Contains('frozen projection changed')) 'Negative baseline lost'
Check ((Log 'preconsent-launch-before-transfer').output.Contains("cannot access 'run.sh'")) 'Assembly failure lost'
foreach($lane in @('194','188')){
 $raw=Get-Content -LiteralPath (Join-Path $PSScriptRoot ("matrix-$lane.log")) -Raw
 $rows=[regex]::Matches($raw,'HC074-PRECONSENT-DRIFT cut=([0-6]) kind=([0-4]) frozen=true refused=true fence_retained=true')
 Check ($rows.Count -eq 35) 'Drift count differs'
 for($cut=0;$cut -lt 7;$cut++){for($kind=0;$kind -lt 5;$kind++){Check (@($rows|Where-Object {$_.Groups[1].Value -eq "$cut" -and $_.Groups[2].Value -eq "$kind"}).Count -eq 1) 'Missing drift'}}
 foreach($kind in 0..4){Check ([regex]::Matches($raw,("HC074-PRECONSENT-REFERENCE kind=$kind refused=true")).Count -eq 1) 'Reference refusal absent'}
 foreach($kind in 0..2){Check ([regex]::Matches($raw,("HC074-PRECONSENT-CONSENT kind=$kind refused=true retained=true")).Count -eq 1) 'Consent refusal absent'}
 foreach($s in @('HC074-PRECONSENT-CONSTRUCTION pre_refused=true fence_retained=true','HC074-PRECONSENT-HEALTHY frozen=true ordered=true same_receiver=true','HC074-PRECONSENT-CONCURRENT closing_round_refused=true fence_retained=true','test result: ok. 1 passed; 0 failed')){Check ($raw.Contains($s)) ('Missing semantic control: '+$s)}
 Check ([regex]::Matches($raw,'HC074-PRECONSENT-ID refused=true retained=true').Count -eq 2) 'ID coverage differs'
 Check ((Log ('preconsent-docs-'+$lane)).output.Contains('6 passed; 0 failed')) 'Compile-fail coverage differs'
 Check ((Log ('preconsent-api-final-'+$lane)).output.Contains('1 passed; 0 failed')) 'Positive named-owner API missing'
 foreach($n in @(24,20,5,7,6)){Check ((Log ('preconsent-regression-'+$lane)).output.Contains("$n passed; 0 failed")) 'Regression coverage differs'}
 Check ((Log ('preconsent-fast-'+$lane)).output.Contains('preconsent_age seed=0x07409c01 cases=3072 exclusive=true')) 'Seeded age proof absent'
 Check ((Log 'preconsent-server-run').output.Contains("lane=$lane exit=0 publication=false")) 'Server result absent'
}
Check ((Log 'preconsent-static-final').output.Contains('185 passed; 0 failed') -and (Log 'preconsent-registry').output.Contains('178 discovered gates, all registered')) 'Static/registry differ'
$run=Get-Content -LiteralPath (Join-Path $PSScriptRoot 'run.sh') -Raw
foreach($b in $m.binaries){Check ($run.Contains($b.sha256+'  '+$b.name)) 'Binary runner pin differs'}
Check ($m.binaries.Count -eq 2) 'Binary inventory differs'
Check ([regex]::Matches((Log 'preconsent-server-run').output,'preflight=ok supervisor_pid=8839 restarts=0').Count -eq 5) 'Host pre/post inventory differs'
$repo=(Resolve-Path -LiteralPath (Join-Path $PSScriptRoot '../../../../../..')).Path
Push-Location $repo
try {
 Check ($m.source_blobs.Count -eq 9) 'Source inventory differs'
 foreach($f in $m.source_blobs){$b=& git rev-parse ($m.source_commit+':'+$f.path);Check ($LASTEXITCODE -eq 0 -and $b -ceq $f.git_blob) 'Source blob differs'}
 & git diff --quiet $m.baseline_commit $m.source_commit -- docs/testing/performance/0.74/qualification-manifest.toml tools/long-run-supervisor-074/src/diagnostic_lease.rs tools/long-run-supervisor-074/src/diagnostic_validation_frame.rs
 Check ($LASTEXITCODE -eq 0) 'Original deadlines/frame/qualification changed'
} finally {Pop-Location}
'Preconsent semantic proof intact: 48 cases/compiler PASS; no publisher, SIGKILL or server admission claim'
