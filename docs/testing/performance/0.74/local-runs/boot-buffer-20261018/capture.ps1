param([Parameter(Mandatory)][string]$SourceCommit)
$ErrorActionPreference='Stop'
$repo=(Resolve-Path (Join-Path $PSScriptRoot '../../../../../..')).Path
if ($SourceCommit -cnotmatch '^[a-f0-9]{40}$') {throw 'source commit required'}
. (Join-Path $PSScriptRoot 'rows.ps1')
$paths=@(
 'tools/long-run-supervisor-074/src/diagnostic_boot_document.rs',
 'tools/long-run-supervisor-074/src/diagnostic_boot_buffer_tests.rs',
 'tools/long-run-supervisor-074/src/diagnostic_kernel_origin.rs',
 'tools/long-run-supervisor-074/tests/diagnostic_boot_buffer_allocations.rs',
 'crates/xtask/tests/support/performance_contract_074_boot_buffer.rs',
 'crates/xtask/tests/support/performance_contract_074_kernel_walk.rs',
 'docs/testing/performance/0.74/local-runs/boot-buffer-20261018/check.ps1'
)
$sources=@()
for ($i=0; $i -lt $paths.Count; $i++) {
    $path=$paths[$i]
    $copy='source-{0:D2}.txt' -f $i
    Copy-Item -LiteralPath (Join-Path $repo $path) -Destination (Join-Path $PSScriptRoot $copy)
    $sources+=@{path=$path;capsule=$copy}
}
$inputRoot=Join-Path $repo 'target/boot-buffer-20261018'
$checks=Join-Path $PSScriptRoot 'checks'
New-Item -ItemType Directory -Force -Path $checks | Out-Null
Get-ChildItem -LiteralPath (Join-Path $inputRoot 'checks') -File | ForEach-Object {Copy-Item -LiteralPath $_.FullName -Destination (Join-Path $checks $_.Name)}
foreach ($log in @('red.log','red-behavior.log')) {Copy-Item -LiteralPath (Join-Path $inputRoot $log) -Destination (Join-Path $PSScriptRoot $log)}
$observations=@()
$images=@()
foreach ($lane in @('windows','194','188')) {
    $log=if ($lane -ceq 'windows') {'windows-allocation.log'} else {"gnu-$lane-allocation.log"}
    $rows=Read-AllocationRows ([IO.File]::ReadAllText((Join-Path $checks $log)))
    $observations+=@{lane=$lane;rows=$rows;timing_claim=$false;whole_worker=$false}
    if ($lane -ceq 'windows') {
        $raw=& cargo test --release -p hydracache-long-run-supervisor-074 --test diagnostic_boot_buffer_allocations --locked --no-run --message-format=json 2> (Join-Path $PSScriptRoot "image-$lane.stderr.log")
    } else {
        $version=if ($lane -ceq '194') {'1.94.0'} else {'1.88.0'}
        $cache="/home/artur/.cache/hydracache-074-$lane-check"
        $raw=& wsl -d Ubuntu -- env "CARGO_TARGET_DIR=$cache" /home/artur/.cargo/bin/cargo "+$version" test --release -p hydracache-long-run-supervisor-074 --test diagnostic_boot_buffer_allocations --locked --no-run --message-format=json 2> (Join-Path $PSScriptRoot "image-$lane.stderr.log")
    }
    $code=$LASTEXITCODE
    $raw | Set-Content -Encoding utf8NoBOM (Join-Path $PSScriptRoot "image-$lane.jsonl")
    if ($code -ne 0) {throw 'allocation image metadata'}
    $selected=@($raw | ForEach-Object {$_ | ConvertFrom-Json} | Where-Object {$_.reason -ceq 'compiler-artifact' -and $_.target.name -ceq 'diagnostic_boot_buffer_allocations' -and $_.profile.test -ceq $true -and $_.executable})
    if ($selected.Count -ne 1 -or $selected[0].fresh -cne $true) {throw 'allocation image changed since execution'}
    $image=[string]$selected[0].executable
    if ($lane -ceq 'windows') {$hash=(Get-FileHash -LiteralPath $image -Algorithm SHA256).Hash.ToLowerInvariant()} else {
        $line=(& wsl -d Ubuntu -- sha256sum -- $image) -join "`n"
        if ($LASTEXITCODE -ne 0 -or $line -cnotmatch '^([a-f0-9]{64})  ') {throw 'image hash'}
        $hash=$Matches[1]
    }
    $images+=@{lane=$lane;path=$image;sha256=$hash;fresh_after_execution=$true}
}
@{source_commit=$SourceCommit;sources=$sources;allocation_images=$images;scope='selected source slice, not a compiler input closure'} | ConvertTo-Json -Depth 8 | Set-Content -Encoding utf8NoBOM (Join-Path $PSScriptRoot 'source-binding.json')
$observations | ConvertTo-Json -Depth 8 | Set-Content -Encoding utf8NoBOM (Join-Path $PSScriptRoot 'observations.json')
$files=@(Get-ChildItem -LiteralPath $PSScriptRoot -Recurse -Force -File | Where-Object {$_.Name -cne 'manifest.json'} | Sort-Object FullName | ForEach-Object {@{path=[IO.Path]::GetRelativePath($PSScriptRoot,$_.FullName).Replace('\','/');sha256=(Get-FileHash -LiteralPath $_.FullName -Algorithm SHA256).Hash.ToLowerInvariant()}})
@{schema='hc074-boot-buffer-local-v1';source_commit=$SourceCommit;baseline='3b8e73feec8412e4090bf05575f06a05901e1010';approved_preparation_source='2ed04dc866b39fefea8d81be2328f7f1382e3d46';status='experimental-local-only';runtime_accepted=$false;host_executed=$false;full_preparation_passed=$false;timing_claim=$false;product_workload=$false;qualification_manifest_sha256='11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc';files=$files} | ConvertTo-Json -Depth 8 | Set-Content -Encoding utf8NoBOM (Join-Path $PSScriptRoot 'manifest.json')
