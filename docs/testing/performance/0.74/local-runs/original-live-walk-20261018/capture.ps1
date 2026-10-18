param([Parameter(Mandatory)][string]$SourceCommit)
$ErrorActionPreference='Stop'
$repo=(Resolve-Path (Join-Path $PSScriptRoot '../../../../../..')).Path
if ($SourceCommit -cnotmatch '^[a-f0-9]{40}$') {throw 'source commit'}
function TextHash([string]$Text) {[Convert]::ToHexString([Security.Cryptography.SHA256]::HashData([Text.Encoding]::UTF8.GetBytes($Text))).ToLowerInvariant()}
function FileHash([string]$Path) {(Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant()}
$sourcePaths=@(
    'crates/xtask/tests/performance_contract_074.rs',
    'crates/xtask/tests/support/performance_contract_074_live_walk.rs',
    'crates/xtask/tests/support/performance_contract_074_original_manager_join.rs',
    'crates/xtask/tests/support/performance_contract_074_preparation_budget.rs',
    'crates/xtask/tests/support/performance_contract_074_safety_alarm.rs',
    'docs/testing/gated-test-registry.toml',
    'tools/long-run-supervisor-074/src/diagnostic_controller_component_cost_tests.rs',
    'tools/long-run-supervisor-074/src/diagnostic_kernel_admission.rs',
    'tools/long-run-supervisor-074/src/diagnostic_validation_trace.rs',
    'tools/long-run-supervisor-074/src/diagnostic_worker_context.rs',
    'tools/long-run-supervisor-074/src/diagnostic_worker_kernel_binding.rs',
    'tools/long-run-supervisor-074/src/diagnostic_worker_trust_input.rs',
    'tools/long-run-supervisor-074/src/diagnostic_live_walk_trace.rs',
    'tools/long-run-supervisor-074/src/diagnostic_original_live_walk_cost_tests.rs'
)
$capsules=Join-Path $PSScriptRoot 'source-capsule'
New-Item -ItemType Directory -Path $capsules -Force | Out-Null
$sources=@(foreach ($path in $sourcePaths) {
    $source=Join-Path $repo $path
    $capsule=Join-Path $capsules ($path.Replace('/','__')+'.source')
    if (!(Test-Path -LiteralPath $capsule)) {Copy-Item -LiteralPath $source -Destination $capsule}
    if ((FileHash $source) -cne (FileHash $capsule)) {throw 'immutable source capsule drift'}
    @{path=$path;raw_sha256=(FileHash $capsule);lf_sha256=(TextHash ([IO.File]::ReadAllText($source).Replace("`r`n","`n")))}
})
$inputPaths=@(Get-ChildItem -LiteralPath (Join-Path $repo 'tools/long-run-supervisor-074/src') -Recurse -File -Filter '*.rs' | ForEach-Object {[IO.Path]::GetRelativePath($repo,$_.FullName).Replace('\','/')})
$inputPaths+=@('Cargo.toml','Cargo.lock','tools/long-run-supervisor-074/Cargo.toml')
$literalCalls=0
$embeddedPaths=@(foreach ($f in Get-ChildItem -LiteralPath (Join-Path $repo 'tools/long-run-supervisor-074/src') -Recurse -File -Filter '*.rs') {
    $raw=[IO.File]::ReadAllText($f.FullName)
    $matches=[regex]::Matches($raw,'\binclude_(?:str|bytes)!\s*\(\s*"([^"]+)"\s*,?\s*\)')
    if ($matches.Count -ne [regex]::Matches($raw,'\binclude_(?:str|bytes)!\s*\(').Count) {throw 'nonliteral embedded input'}
    $literalCalls+=$matches.Count
    foreach ($m in $matches) {[IO.Path]::GetRelativePath($repo,[IO.Path]::GetFullPath((Join-Path $f.DirectoryName $m.Groups[1].Value))).Replace('\','/')}
})
$embeddedPaths=@($embeddedPaths | Sort-Object -Unique)
if ($literalCalls -ne 27 -or $embeddedPaths.Count -ne 12) {throw 'embedded census'}
$inputPaths=@($inputPaths+$embeddedPaths | Sort-Object -Unique)
$embeddedCapsules=Join-Path $PSScriptRoot 'embedded-capsule'
New-Item -ItemType Directory -Path $embeddedCapsules -Force | Out-Null
$embedded=@(foreach ($path in $embeddedPaths) {
    $source=Join-Path $repo $path
    $capsule=Join-Path $embeddedCapsules ($path.Replace('/','__')+'.source')
    if (!(Test-Path -LiteralPath $capsule)) {Copy-Item -LiteralPath $source -Destination $capsule}
    if ((FileHash $source) -cne (FileHash $capsule)) {throw 'embedded byte drift'}
    @{path=$path;raw_sha256=(FileHash $capsule);lf_sha256=(TextHash ([IO.File]::ReadAllText($source).Replace("`r`n","`n")))}
})
$inputs=@(foreach ($path in ($inputPaths | Sort-Object)) {
    @{path=$path;lf_sha256=(TextHash ([IO.File]::ReadAllText((Join-Path $repo $path)).Replace("`r`n","`n")))}
})
if ($sources.Count -ne 14 -or $inputs.Count -ne 362) {throw 'source inventory'}
@{baseline_commit='b2adb5b73a2f66fc690637260c100b5c8489aa35';preregistration_commit='d5cb0b0e';source_commit=$SourceCommit;sources=$sources;build_inputs=$inputs;literal_embed_calls=$literalCalls;embedded_inputs=$embedded} | ConvertTo-Json -Depth 8 | Set-Content -Encoding utf8NoBOM (Join-Path $PSScriptRoot 'selected-sources.json')
$files=@(Get-ChildItem -LiteralPath $PSScriptRoot -Recurse -Force -File | Where-Object {$_.Name -cne 'manifest.json'} | ForEach-Object {
    @{path=[IO.Path]::GetRelativePath($PSScriptRoot,$_.FullName).Replace('\','/');sha256=(FileHash $_.FullName);bytes=$_.Length}
} | Sort-Object path)
@{schema='hc074-original-live-walk-evidence-v1';status='accepted-diagnostic';source_commit=$SourceCommit;qualification_manifest_sha256='11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc';start_admitted=$false;measurement_admitted=$false;installation_performed=$false;supervisor_changed=$false;files=$files} | ConvertTo-Json -Depth 8 | Set-Content -Encoding utf8NoBOM (Join-Path $PSScriptRoot 'manifest.json')
'HC074-LIVE-WALK-CAPTURE sources=14 inputs=362 embedded=12 admission=false'
