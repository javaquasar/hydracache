$ErrorActionPreference='Stop'
$root=(Resolve-Path (Join-Path $PSScriptRoot '../../../../../..')).Path
$base='d506d4d94f673e4b1c3ed59874074ffd693dc37b'
function HashText([string]$Text) {[Convert]::ToHexString([Security.Cryptography.SHA256]::HashData([Text.Encoding]::UTF8.GetBytes($Text))).ToLowerInvariant()}
$paths=@(
    'crates/xtask/tests/support/performance_contract_074_reply_flags.rs',
    'crates/xtask/tests/support/performance_contract_074_original_manager_message.rs',
    'docs/COMPAT.md','docs/testing/gated-test-registry.toml'
)
$names=@('diagnostic_dispatch_exchange_socket_tests.rs','diagnostic_original_manager_frame_tests.rs','diagnostic_original_manager_join.rs','diagnostic_original_manager_join_tests.rs','diagnostic_original_addressed_transfer_tests.rs','diagnostic_original_manager_message.rs','diagnostic_original_manager_message_frame_tests.rs','diagnostic_original_manager_message_red.rs','diagnostic_original_manager_message_test_api.rs','diagnostic_original_manager_message_units.rs')
$paths+=@($names | ForEach-Object {'tools/long-run-supervisor-074/src/'+$_})
$capsules=Join-Path $PSScriptRoot 'source-capsule'
New-Item -ItemType Directory -Path $capsules -Force | Out-Null
$sources=@(foreach($path in $paths) {
    $source=Join-Path $root $path
    $capsule=Join-Path $capsules ($path.Replace('/','__')+'.source')
    if(Test-Path -LiteralPath $capsule){throw 'source snapshot already exists'}
    Copy-Item -LiteralPath $source -Destination $capsule
    @{path=$path;raw_sha256=(Get-FileHash -LiteralPath $capsule -Algorithm SHA256).Hash.ToLowerInvariant();lf_sha256=(HashText ([IO.File]::ReadAllText($capsule).Replace("`r`n","`n")))}
})
$inputs=@(Get-ChildItem -LiteralPath (Join-Path $root 'tools/long-run-supervisor-074/src') -File -Filter '*.rs' | ForEach-Object {'tools/long-run-supervisor-074/src/'+$_.Name})
$inputs+=@('Cargo.toml','Cargo.lock','tools/long-run-supervisor-074/Cargo.toml')
$bindings=@(foreach($path in ($inputs | Sort-Object)) {
    @{path=$path;lf_sha256=(HashText ([IO.File]::ReadAllText((Join-Path $root $path)).Replace("`r`n","`n")))}
})
if($sources.Count -ne 14 -or $bindings.Count -ne 346){throw 'source inventory'}
@{base_commit=$base;source_files=$sources;build_inputs=$bindings} | ConvertTo-Json -Depth 8 | Set-Content -Encoding utf8NoBOM (Join-Path $PSScriptRoot 'selected-sources.json')
'HC074-GROUPED-SOURCE sources=14 inputs=346 snapshot=true'
