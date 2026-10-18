param([switch]$SelfTest)
$ErrorActionPreference = 'Stop'
$packet = $PSScriptRoot
$root = (Resolve-Path (Join-Path $packet '../../../../../..')).Path
function Require([bool]$Condition,[string]$Reason) {if (-not $Condition) {throw $Reason}}
function HashText([string]$Value) {
    [Convert]::ToHexString([Security.Cryptography.SHA256]::HashData([Text.Encoding]::UTF8.GetBytes($Value))).ToLowerInvariant()
}
function GitText([string]$Ref,[string]$Path) {
    $start=[Diagnostics.ProcessStartInfo]::new('git')
    $start.WorkingDirectory=$root; $start.UseShellExecute=$false
    $start.RedirectStandardOutput=$true; $start.StandardOutputEncoding=[Text.UTF8Encoding]::new($false)
    $start.ArgumentList.Add('show'); $start.ArgumentList.Add("${Ref}:$Path")
    $process=[Diagnostics.Process]::Start($start)
    $value=$process.StandardOutput.ReadToEnd(); $process.WaitForExit()
    Require ($process.ExitCode -eq 0) 'git source unavailable'
    $value
}
function Count([string]$Raw,[string]$Needle) {[regex]::Matches($Raw,[regex]::Escape($Needle)).Count}
function CheckManifest($m) {
    Require ($m.schema -ceq 'hc074-dispatch-exchange-v1' -and $m.status -ceq 'local-message-mechanics-only') 'schema/scope'
    foreach ($field in @('admission','original_start','manager_authenticated','writer_cessation')) {
        Require ($m.$field -ceq $false) "false $field"
    }
    Require ($m.unit_tests_per_compiler -eq 14 -and $m.source_contracts -eq 229) 'selected gate'
    Require ($m.source_files.Count -eq 7 -and $m.build_inputs.Count -eq 329) 'source inventory'
    Require ($m.source_commit -cmatch '^[a-f0-9]{40}$' -and $m.base_commit -cmatch '^6af23de5[a-f0-9]{32}$') 'source identity'
    Require ($m.implementation_commit -ceq '642265063c8bba31f4f71894b366f5cb53d4638e') 'implementation identity'
    Require ($m.qualification_sha256 -ceq '11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc') 'frozen qualification'
}
function CheckTests([string]$Raw) {
    Require ($Raw -cnotmatch 'FAILED|panicked|error\[') 'unexpected test failure'
    Require ((Count $Raw 'test result: ok. 14 passed; 0 failed; 0 ignored;') -eq 1) 'test summary'
    foreach ($name in @(
        'dispatch_real_rights_roundtrip_correlates_job_without_starting_a_process',
        'dispatch_actual_delivery_with_lost_reply_preserves_writers_and_cannot_retry',
        'dispatch_delayed_reply_cannot_heal_a_closed_unknown_outcome',
        'dispatch_job_and_drop_do_not_prove_cessation_of_hidden_message_aliases')) {
        Require ((Count $Raw ($name+' ... ok')) -eq 1) "selected row $name"
    }
}
$m=Get-Content (Join-Path $packet 'manifest.json') -Raw | ConvertFrom-Json
CheckManifest $m
$actual=@(Get-ChildItem -LiteralPath $packet -Recurse -File | ForEach-Object {
    [IO.Path]::GetRelativePath($packet,$_.FullName).Replace('\','/')
} | Where-Object {$_ -ne 'manifest.json'} | Sort-Object)
Require (($actual -join "`n") -ceq (($m.files.path | Sort-Object) -join "`n")) 'closed inventory'
foreach ($file in $m.files) {
    Require ($file.path -cnotmatch '(^/|\\|\.\.)') 'unsafe packet path'
    Require ((Get-FileHash -LiteralPath (Join-Path $packet $file.path) -Algorithm SHA256).Hash.ToLowerInvariant() -ceq $file.sha256) "file digest $($file.path)"
}
$sources=@{}
foreach ($file in $m.source_files) {
    Require (-not $sources.ContainsKey($file.path)) 'duplicate capsule'
    $name=Join-Path $packet ('source-capsule/'+$file.path.Replace('/','__')+'.source')
    Require ((Get-FileHash -LiteralPath $name -Algorithm SHA256).Hash.ToLowerInvariant() -ceq $file.raw_sha256) 'raw capsule'
    $lf=[IO.File]::ReadAllText($name).Replace("`r`n","`n")
    Require ((HashText $lf) -ceq $file.lf_sha256) 'canonical capsule'
    Require ((GitText $m.source_commit $file.path).Replace("`r`n","`n") -ceq $lf) 'accepted source differs'
    $sources[$file.path]=$lf
}
$changed=@(git -C $root diff --name-only $m.base_commit $m.source_commit)
Require ($LASTEXITCODE -eq 0) 'source diff unavailable'
Require ((($changed | Sort-Object) -join "`n") -ceq (($m.source_files.path | Sort-Object) -join "`n")) 'changes outside capsule'
$parent=(git -C $root rev-parse "$($m.source_commit)^").Trim()
Require ($LASTEXITCODE -eq 0 -and $parent -ceq $m.implementation_commit) 'source parent differs'
$priorParent=(git -C $root rev-parse "$($m.implementation_commit)^").Trim()
Require ($LASTEXITCODE -eq 0 -and $priorParent -ceq $m.base_commit) 'implementation parent differs'
$reviewChanges=@(git -C $root diff --name-only $m.implementation_commit $m.source_commit)
Require ($LASTEXITCODE -eq 0 -and $reviewChanges.Count -eq 1 -and
    $reviewChanges[0] -ceq 'tools/long-run-supervisor-074/src/diagnostic_dispatch_exchange_tests.rs') 'review is not test only'
$inputPaths=@{}
foreach ($file in $m.build_inputs) {
    Require (-not $inputPaths.ContainsKey($file.path)) 'duplicate input'
    $inputPaths[$file.path]=$true
    $lf=if ($sources.ContainsKey($file.path)) {$sources[$file.path]} else {GitText $m.base_commit $file.path}
    Require ((HashText $lf) -ceq $file.lf_sha256) "build input $($file.path)"
}
$rows=@($m.build_inputs | ForEach-Object {"$($_.raw_sha256)  $($_.path)"})
Require ([IO.File]::ReadAllText((Join-Path $packet 'build-inputs.log')) -ceq (($rows -join "`n")+"`n")) 'raw input binding'
foreach ($compiler in @('194','188')) {
    CheckTests ([IO.File]::ReadAllText((Join-Path $packet "final-tests-$compiler.log")))
    $raw=[IO.File]::ReadAllText((Join-Path $packet "gnu-$compiler.log"))
    Require ($raw -cnotmatch 'FAILED|panicked|error\[') 'GNU failure'
    foreach ($n in @(8,32,2)) {Require ($raw.Contains("test result: ok. $n passed; 0 failed;")) 'regression/doctest inventory'}
    Require ($raw.Contains("HC074-DISPATCH-GNU compiler=$compiler completed=true admission=false")) 'GNU gate incomplete'
    $hash=(Get-Content (Join-Path $packet "binary-$compiler.sha256") -Raw).Trim()
    Require ($hash -cmatch '^[a-f0-9]{64}$' -and (Count $raw "$hash  /home/artur/.cache/hydracache-074-$compiler-check/release/deps/") -eq 2) 'image identity'
}
Require ((Get-Content (Join-Path $packet 'contracts.log') -Raw).Contains('test result: ok. 229 passed;')) 'contracts'
Require ((Get-Content (Join-Path $packet 'final-source-contracts.log') -Raw).Contains('test result: ok. 4 passed;')) 'final mutation canaries'
Require ((Get-Content (Join-Path $packet 'windows.log') -Raw).Contains('test result: ok. 44 passed;')) 'portable controls'
Require ((Get-Content (Join-Path $packet 'windows.log') -Raw).Contains('HC074-DISPATCH-WINDOWS completed=true admission=false')) 'Windows check/Clippy'
Require ((Get-Content (Join-Path $packet 'local-checks.log') -Raw).Contains('HC074-DISPATCH-LOCAL completed=true admission=false')) 'local gates'
Require ((Get-Content (Join-Path $packet 'review-format.log') -Raw).Contains('HC074-DISPATCH review_format=true')) 'final test formatting'
$docs=Get-Content (Join-Path $packet 'documentation-final.log') -Raw
Require ($docs.Contains('doc-check: OK') -and $docs.Contains('performance-contract-check 0.74: OK (local, non-promotable)')) 'final documentation gates'
Require ((Get-Content (Join-Path $packet 'previous-packet-verification.log') -Raw).Contains('HC074-ENROLLMENT-FRAME-OFFLINE positive=1 negatives=8 selected_gate=true admission=false')) 'previous packet preservation'
Require ((Get-Content (Join-Path $packet 'missing-api.log') -Raw).Contains('error[E0425]')) 'missing API red proof'
$first=Get-Content (Join-Path $packet 'first-implementation-refusal.log') -Raw
Require ($first.Contains('test result: FAILED. 4 passed; 6 failed;')) 'first implementation negative result'
if ($SelfTest) {
    foreach ($field in @('admission','original_start','manager_authenticated','writer_cessation')) {
        $bad=$m | ConvertTo-Json -Depth 12 | ConvertFrom-Json
        $bad.$field=$true; $rejected=$false
        try {CheckManifest $bad} catch {$rejected=$true}
        Require $rejected "negative $field"
    }
    $bad=$m | ConvertTo-Json -Depth 12 | ConvertFrom-Json
    $bad.build_inputs=@($bad.build_inputs | Select-Object -Skip 1); $rejected=$false
    try {CheckManifest $bad} catch {$rejected=$true}
    Require $rejected 'negative input inventory'
    $raw=Get-Content (Join-Path $packet 'final-tests-194.log') -Raw
    $rejected=$false
    try {CheckTests ($raw.Replace('dispatch_delayed_reply_cannot_heal_a_closed_unknown_outcome ... ok','removed'))} catch {$rejected=$true}
    Require $rejected 'negative missing delayed reply case'
    Require ((HashText ($sources['tools/long-run-supervisor-074/src/diagnostic_dispatch_exchange.rs']+'mutation')) -cne
        ($m.source_files | Where-Object {$_.path -ceq 'tools/long-run-supervisor-074/src/diagnostic_dispatch_exchange.rs'}).lf_sha256) 'negative source mutation'
    'HC074-DISPATCH negative_controls=7 rejected=true'
}
'HC074-DISPATCH verified=true admission=false original_start=false writer_cessation=false'
