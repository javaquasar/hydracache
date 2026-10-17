$ErrorActionPreference = 'Stop'
$packet = $PSScriptRoot
$repo = (& git -C $packet rev-parse --show-toplevel).Trim()
if ($LASTEXITCODE -ne 0) { throw 'Repository unavailable' }
function Require($value, [string]$why) { if (-not $value) { throw $why } }
function Scope($s) {
    foreach ($key in @('material_complete','deadlines_unchanged','original_guards_unchanged','admission_closed')) {
        Require (($s.$key -is [bool]) -and $s.$key) "Missing boundary $key"
    }
    foreach ($key in @('authenticated_start','writer_handoff','all_writer_cessation','completion','later_cells','production_enrollment','installed_release_changed','supervisor_stopped','clock_changed','product_workload','qualification_changed','performance_claim','server_ready')) {
        Require (($s.$key -is [bool]) -and -not $s.$key) "Invalid admission $key"
    }
}
function Count([string]$raw, [string]$pattern, [int]$want) {
    Require ([regex]::Matches($raw, $pattern, [Text.RegularExpressions.RegexOptions]::Multiline).Count -eq $want) "Count mismatch $pattern"
}
function Blob([string]$raw) {
    $bytes = [Text.Encoding]::UTF8.GetBytes($raw)
    $header = [Text.Encoding]::UTF8.GetBytes("blob $($bytes.Length)" + [char]0)
    $sha = [Security.Cryptography.SHA1]::Create()
    try { return ([BitConverter]::ToString($sha.ComputeHash($header + $bytes))).Replace('-','').ToLowerInvariant() }
    finally { $sha.Dispose() }
}
function Faults([string]$raw) {
    $cases = @('load-content','load-missing','closing-trust','closing-output','later-content','later-replace','later-trust','later-output','projection','starting','descriptor','fixture-entry','expiry')
    foreach ($case in $cases) {
        Count $raw ("HC074-START-MATERIAL-REFUSAL case=" + $case + ' exact=true sticky=true held=true unchanged=true start=false') 1
    }
    Count $raw 'HC074-START-MATERIAL-REFUSAL case=' 13
    Require $raw.Contains('1 passed; 0 failed;') 'Fault family did not pass'
    $clock = [regex]::Match($raw, 'HC074-START-MATERIAL-EXPIRY event=Event \{[^\r\n]*site: "progress-age"[^\r\n]*clock: \[(\d+), (\d+), (\d+), 0, 0, (\d+)\][^\r\n]*limit_ns=10000000000 renewed=false')
    Require $clock.Success 'Exact original expiry event missing'
    $received = [UInt64]$clock.Groups[1].Value
    $after = [UInt64]$clock.Groups[2].Value
    $now = [UInt64]$clock.Groups[3].Value
    $limit = [UInt64]$clock.Groups[4].Value
    Require ($received -gt $after -and $now -ge $received -and $limit -eq 10000000000 -and $now - $received -ge $limit) 'Expiry guard changed'
}
$m = Get-Content -LiteralPath (Join-Path $packet 'manifest.json') -Raw | ConvertFrom-Json
Require ($m.baseline_commit -eq '2c63cac08f70b18e4af78875f1f95c35c16238f0') 'Wrong baseline'
Require ($m.preregister_commit -eq 'c0fed67cb486a9c8a03c341abcee38e3f13f9a95' -and $m.attribution_preregister_commit -eq '2fbee91b37111c86e43a3c7d436a62768acbc6b3') 'Wrong preregistration'
Scope $m.scope
Require ($m.implementation_commit -eq 'fe0c22b234e593b3bf79cbb99f504b517344421a') 'Wrong implementation commit'
$bad = $m.scope | ConvertTo-Json | ConvertFrom-Json
$bad.server_ready = $true
$rejected = $false
try { Scope $bad } catch { $rejected = $true }
Require $rejected 'False-admission canary survived'
$seen = @{}
foreach ($a in $m.artifacts) {
    Require ($a.path -notmatch '(^/|^[A-Za-z]:|\\|\.\.)') 'Unsafe artifact path'
    Require (-not $seen.ContainsKey($a.path)) 'Duplicate artifact'
    $seen[$a.path] = $true
    $path = Join-Path $packet $a.path
    Require (Test-Path -LiteralPath $path -PathType Leaf) "Missing $($a.path)"
    Require ((Get-Item -LiteralPath $path).Length -eq $a.bytes) "Length changed $($a.path)"
    Require ((Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash.ToLowerInvariant() -eq $a.sha256) "Hash changed $($a.path)"
}
$actual = @(Get-ChildItem -LiteralPath $packet -File -Recurse | ForEach-Object { $_.FullName.Substring($packet.Length + 1).Replace('\','/') } | Where-Object { $_ -notin @('README.md','manifest.json','verify.ps1') } | Sort-Object)
Require (($actual -join '|') -eq (($m.artifacts.path | Sort-Object) -join '|')) 'Artifact inventory changed'
& (Join-Path $packet 'verify-source-capsules.ps1')
Require ($m.source_blobs.Count -eq 10) 'Wrong source inventory'
foreach ($s in $m.source_blobs) {
    Require ($s.path -notmatch '(^/|^[A-Za-z]:|\\|\.\.)') 'Unsafe source path'
    $raw = [IO.File]::ReadAllText((Join-Path $repo $s.path)).Replace([string][char]13,'')
    Require ((Blob $raw) -eq $s.git_blob) "Active source changed $($s.path)"
    $committed = (& git -C $repo rev-parse ($m.implementation_commit + ':' + $s.path)).Trim()
    Require ($LASTEXITCODE -eq 0 -and $committed -eq $s.git_blob) "Committed implementation differs $($s.path)"
}
$prior = Join-Path $repo 'docs/testing/performance/0.74/local-runs/original-collector-sinks-join-20261017'
foreach ($s in (Get-Content -LiteralPath (Join-Path $prior 'unchanged-runtime.json') -Raw | ConvertFrom-Json)) {
    $raw = [IO.File]::ReadAllText((Join-Path $repo $s.path)).Replace([string][char]13,'')
    Require ((Blob $raw) -eq $s.git_blob) "Frozen original changed $($s.path)"
}
$old = Get-Content -LiteralPath (Join-Path $prior 'manifest.json') -Raw | ConvertFrom-Json
foreach ($s in $old.runtime_blobs) {
    $raw = [IO.File]::ReadAllText((Join-Path $repo $s.path)).Replace([string][char]13,'')
    if ($s.path.EndsWith('/diagnostic_original_collector_sinks.rs')) {
        $registration = '#[path = "diagnostic_original_start_material.rs"]' + [char]10 + 'pub mod start_material;' + [char]10
        Count $raw ([regex]::Escape($registration)) 1
        $raw = $raw.Replace($registration, '')
    }
    Require ((Blob $raw) -eq $s.git_blob) "Original output runtime changed $($s.path)"
}
$red = [IO.File]::ReadAllText((Join-Path $packet 'healthy-194.log'))
Require ($red.Contains('0 passed; 1 failed;') -and $red.Contains('Causal(Time)')) 'First red observation omitted'
Count $red 'HC074-START-MATERIAL-CONTROL sample=' 0
$cost = [IO.File]::ReadAllText((Join-Path $packet 'attribution-194.log'))
foreach ($sample in 0..2) {
    Count $cost ("HC074-START-MATERIAL-COST sample=$sample phase=outputs [^\r\n]*authentication_calls: 782,") 1
    Count $cost ("HC074-START-MATERIAL-COST sample=$sample phase=material [^\r\n]*authentication_calls: 68,") 1
    Count $cost ("HC074-START-MATERIAL-COST sample=$sample phase=revalidate [^\r\n]*authentication_calls: 51,[^\r\n]*result=Err\(Outputs\(Original\(Original\(Original\(Causal\(Time\)\)\)\)\)\) event=None") 1
    Count $cost ("HC074-START-MATERIAL-COST-CLOSED sample=$sample exact_refusal=true accepted=false renewed=false start=false") 1
}
Require $cost.Contains('1 passed; 0 failed;') 'Attribution incomplete'
foreach ($lane in @('194','188')) {
    $healthy = [IO.File]::ReadAllText((Join-Path $packet "healthy-v2-$lane.log"))
    foreach ($sample in 0..2) {
        Count $healthy ("HC074-START-MATERIAL-CONTROL sample=$sample original=true bundle=true fixture=true retained=true start=false") 1
        foreach ($phase in @('prepare','validate')) {
            Count $healthy ("HC074-START-MATERIAL-V2-COST sample=$sample phase=$phase [^\r\n]*authentication_calls: 34,") 1
        }
    }
    Require $healthy.Contains('1 passed; 0 failed;') 'Healthy family incomplete'
    $faults = [IO.File]::ReadAllText((Join-Path $packet "refusals-v2-$lane.log"))
    Faults $faults
    $first = [regex]::Match($faults, 'HC074-START-MATERIAL-REFUSAL case=[^\r\n]+').Value
    $rejected = $false
    try { Faults ($faults.Replace($first,'')) } catch { $rejected = $true }
    Require $rejected 'Missing case canary survived'
}
$post = [IO.File]::ReadAllText((Join-Path $packet 'postflight.log'))
Require $post.Contains('test_processes=absent supervisor_unchanged=true installed_receipt_unchanged=true active_contexts=absent clock_changed=false product=false') 'Host postflight missing'
Require ($post.Contains('MainPID=8839') -and $post.Contains('NRestarts=0') -and $post.Contains('ActiveState=active') -and $post.Contains('SubState=running')) 'Supervisor identity drift'
& git -C $repo diff --exit-code $m.baseline_commit -- docs/testing/performance/0.74/qualification-manifest.toml
Require ($LASTEXITCODE -eq 0) 'Qualification changed'
Write-Output 'Original material v2 verified: 6 healthy chains, 26 exact refusal cases, first red retained, original live guards/deadlines exact; authenticated start and production admission CLOSED'
