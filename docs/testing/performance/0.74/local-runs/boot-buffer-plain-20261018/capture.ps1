param([Parameter(Mandatory)][string]$SourceCommit)
$ErrorActionPreference='Stop'
$repo=(Resolve-Path (Join-Path $PSScriptRoot '../../../../../..')).Path
if ($SourceCommit -cnotmatch '^[a-f0-9]{40}$') {throw 'source identity'}
$inputRoot=Join-Path $repo 'target/boot-buffer-plain-20261018'
$receipts=Join-Path $PSScriptRoot 'receipts'
$capsules=Join-Path $PSScriptRoot 'source-capsule'
$embedded=Join-Path $PSScriptRoot 'embedded-capsule'
foreach ($dir in @($receipts,$capsules,$embedded)) {New-Item -ItemType Directory -Force -Path $dir | Out-Null}
Get-ChildItem -LiteralPath $inputRoot -File | Where-Object {$_.Extension -cin @('.json','.jsonl','.log','.source') -or $_.Name -ceq 'SHA256SUMS'} | ForEach-Object {Copy-Item -LiteralPath $_.FullName -Destination (Join-Path $receipts $_.Name)}
$checks=Join-Path $receipts 'checks'
New-Item -ItemType Directory -Force -Path $checks | Out-Null
Get-ChildItem -LiteralPath (Join-Path $inputRoot 'checks') -File | ForEach-Object {Copy-Item -LiteralPath $_.FullName -Destination (Join-Path $checks $_.Name)}
$paths=@(& git -C $repo diff --name-only 3279fecf $SourceCommit)
if ($LASTEXITCODE -ne 0 -or $paths.Count -ne 14) {throw 'source change census'}
$sources=@(foreach ($path in $paths) {
    $copy=$path.Replace('/','__')+'.source'
    Copy-Item -LiteralPath (Join-Path $repo $path) -Destination (Join-Path $capsules $copy)
    @{path=$path;capsule="source-capsule/$copy"}
})
$embedPaths=@(foreach ($file in Get-ChildItem -LiteralPath (Join-Path $repo 'tools/long-run-supervisor-074/src') -Recurse -File -Filter '*.rs') {
    foreach ($m in [regex]::Matches([IO.File]::ReadAllText($file.FullName),'\binclude_(?:str|bytes)!\s*\(\s*"([^"]+)"\s*,?\s*\)')) {
        [IO.Path]::GetRelativePath($repo,[IO.Path]::GetFullPath((Join-Path $file.DirectoryName $m.Groups[1].Value))).Replace('\','/')
    }
})
$embedPaths=@($embedPaths | Sort-Object -Unique)
if ($embedPaths.Count -ne 12) {throw 'embedded input census'}
$embeds=@(foreach ($path in $embedPaths) {
    $copy=$path.Replace('/','__')+'.source'
    Copy-Item -LiteralPath (Join-Path $repo $path) -Destination (Join-Path $embedded $copy)
    @{path=$path;capsule="embedded-capsule/$copy"}
})
@{source_commit=$SourceCommit;preregistration_commit=(& git -C $repo rev-parse 3279fecf);sources=$sources;embedded=$embeds;rust_sources=356;selected_inputs=367;literal_embed_calls=27;complete_compiler_attestation=$false} | ConvertTo-Json -Depth 8 | Set-Content -Encoding utf8NoBOM (Join-Path $PSScriptRoot 'source-binding.json')
. (Join-Path $PSScriptRoot 'rows.ps1')
$parsed=Read-HostRows ([IO.File]::ReadAllText((Join-Path $receipts 'host.log')))
$stats=Paired-Statistics $parsed.rows
$accepted=@($stats | Where-Object {!$_.non_regression}).Count -eq 0
@{raw=$parsed;statistics=$stats;preparation_accepted=$accepted;product_claim=$false} | ConvertTo-Json -Depth 12 | Set-Content -Encoding utf8NoBOM (Join-Path $PSScriptRoot 'observations.json')
$files=@(Get-ChildItem -LiteralPath $PSScriptRoot -Recurse -Force -File | Where-Object {$_.Name -cne 'manifest.json'} | ForEach-Object {@{path=[IO.Path]::GetRelativePath($PSScriptRoot,$_.FullName).Replace('\','/');sha256=(Get-FileHash -LiteralPath $_.FullName -Algorithm SHA256).Hash.ToLowerInvariant();bytes=$_.Length}} | Sort-Object path)
@{schema='hc074-boot-buffer-plain-v1';source_commit=$SourceCommit;status=$(if ($accepted) {'accepted-preparation-only'} else {'inconclusive-preparation'});preparation_accepted=$accepted;start_admitted=$false;measurement_admitted=$false;installation_performed=$false;supervisor_changed=$false;qualification_manifest_sha256='11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc';files=$files} | ConvertTo-Json -Depth 8 | Set-Content -Encoding utf8NoBOM (Join-Path $PSScriptRoot 'manifest.json')
