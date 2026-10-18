$ErrorActionPreference='Stop'
$root=(Resolve-Path (Join-Path $PSScriptRoot '../../../../../..')).Path
$base='537e1f253570bdff4e5f07316701a12d4c172da3'
function HashText([string]$Text) {[Convert]::ToHexString([Security.Cryptography.SHA256]::HashData([Text.Encoding]::UTF8.GetBytes($Text))).ToLowerInvariant()}
$paths=@(
    'crates/xtask/tests/support/performance_contract_074_reply_flags.rs',
    'crates/xtask/tests/support/performance_contract_074_preparation_budget.rs',
    'docs/testing/gated-test-registry.toml',
    'tools/long-run-supervisor-074/src/diagnostic_original_manager_message_frame_tests.rs',
    'tools/long-run-supervisor-074/src/diagnostic_original_preparation_budget_tests.rs'
)
$capsules=Join-Path $PSScriptRoot 'source-capsule'
New-Item -ItemType Directory -Path $capsules -Force | Out-Null
$sources=@(foreach($path in $paths) {
    $source=Join-Path $root $path
    $capsule=Join-Path $capsules ($path.Replace('/','__')+'.source')
    if(Test-Path -LiteralPath $capsule){throw 'source snapshot already exists'}
    Copy-Item -LiteralPath $source -Destination $capsule
    @{path=$path;raw_sha256=(Get-FileHash -LiteralPath $capsule -Algorithm SHA256).Hash.ToLowerInvariant();lf_sha256=(HashText ([IO.File]::ReadAllText($capsule).Replace("`r`n","`n")))}
})
$inputs=@(Get-ChildItem -LiteralPath (Join-Path $root 'tools/long-run-supervisor-074/src') -Recurse -File -Filter '*.rs' | ForEach-Object {[IO.Path]::GetRelativePath($root,$_.FullName).Replace('\','/')})
$inputs+=@('Cargo.toml','Cargo.lock','tools/long-run-supervisor-074/Cargo.toml')
$bindings=@(foreach($path in ($inputs | Sort-Object)) {
    @{path=$path;lf_sha256=(HashText ([IO.File]::ReadAllText((Join-Path $root $path)).Replace("`r`n","`n")))}
})
if($sources.Count -ne 5 -or $bindings.Count -ne 352){throw 'source inventory'}
@{base_commit=$base;source_files=$sources;build_inputs=$bindings} | ConvertTo-Json -Depth 8 | Set-Content -Encoding utf8NoBOM (Join-Path $PSScriptRoot 'selected-sources.json')
'HC074-PREPARATION-SOURCE sources=5 inputs=352 snapshot=true'
