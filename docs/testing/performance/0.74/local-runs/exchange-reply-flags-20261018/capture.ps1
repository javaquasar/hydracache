param([Parameter(Mandatory)][string]$SourceCommit,[switch]$Refresh)
$ErrorActionPreference='Stop'
$root=(Resolve-Path (Join-Path $PSScriptRoot '../../../../../..')).Path
if ($SourceCommit -cnotmatch '^[0-9a-f]{40}$') {throw 'source commit'}
$paths=@(git -C $root diff --name-only ff665c3b $SourceCommit)
if ($LASTEXITCODE -ne 0 -or $paths.Count -ne 6) {throw 'source inventory'}
$capsules=Join-Path $PSScriptRoot 'source-capsule'
if (Test-Path -LiteralPath $capsules) {
    if (!$Refresh) {throw 'capsule exists'}
} else {
    New-Item -ItemType Directory -Path $capsules | Out-Null
}
function HashText([string]$Text) {[Convert]::ToHexString([Security.Cryptography.SHA256]::HashData([Text.Encoding]::UTF8.GetBytes($Text))).ToLowerInvariant()}
$sources=@(foreach ($path in $paths) {
    $original=Join-Path $root $path
    $target=Join-Path $capsules ($path.Replace('/','__')+'.source')
    if ($Refresh) {
        if ((Get-FileHash -LiteralPath $original -Algorithm SHA256).Hash -cne (Get-FileHash -LiteralPath $target -Algorithm SHA256).Hash) {throw 'capsule changed'}
    } else {
        Copy-Item -LiteralPath $original -Destination $target
    }
    @{path=$path;raw_sha256=(Get-FileHash -LiteralPath $target -Algorithm SHA256).Hash.ToLowerInvariant();lf_sha256=(HashText ([IO.File]::ReadAllText($target).Replace("`r`n","`n")))}
})
$inputs=@('Cargo.toml','Cargo.lock','tools/long-run-supervisor-074/Cargo.toml')
$inputs+=@(Get-ChildItem -LiteralPath (Join-Path $root 'tools/long-run-supervisor-074/src') -Filter '*.rs' -File | ForEach-Object {'tools/long-run-supervisor-074/src/'+$_.Name})
$bindings=@(foreach ($path in ($inputs | Sort-Object)) {
    @{path=$path;lf_sha256=(HashText ([IO.File]::ReadAllText((Join-Path $root $path)).Replace("`r`n","`n")))}
})
$manifest=@{
    schema='hc074-exchange-reply-flags-v1';base_commit=(git -C $root rev-parse ff665c3b).Trim();source_commit=$SourceCommit
    admission=$false;original_start=$false;writer_cessation=$false;server_install=$false;product_workload=$false
    source_files=$sources;build_inputs=$bindings
    images=@(Get-Content (Join-Path $PSScriptRoot 'Candidate-images.json') -Raw | ConvertFrom-Json)
    qualification_sha256=(Get-FileHash -LiteralPath (Join-Path $root 'docs/testing/performance/0.74/qualification-manifest.toml') -Algorithm SHA256).Hash.ToLowerInvariant()
}
$manifest.files=@(Get-ChildItem -LiteralPath $PSScriptRoot -Recurse -File |
    Where-Object {$_.Name -cne 'manifest.json'} | Sort-Object FullName | ForEach-Object {
        @{path=[IO.Path]::GetRelativePath($PSScriptRoot,$_.FullName).Replace('\','/');sha256=(Get-FileHash -LiteralPath $_.FullName -Algorithm SHA256).Hash.ToLowerInvariant()}
    })
$manifest | ConvertTo-Json -Depth 10 | Set-Content -Encoding utf8NoBOM (Join-Path $PSScriptRoot 'manifest.json')
"HC074-EXCHANGE-CAPTURE sources=$($sources.Count) inputs=$($bindings.Count) source=$SourceCommit"
