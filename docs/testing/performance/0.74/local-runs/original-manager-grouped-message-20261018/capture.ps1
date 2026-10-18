param([Parameter(Mandatory)][ValidateSet('accepted-fixture','withdrawn-deadline')][string]$Status,[string]$SourceCommit)
$ErrorActionPreference='Stop'
$root=(Resolve-Path (Join-Path $PSScriptRoot '../../../../../..')).Path
$selected=Get-Content (Join-Path $PSScriptRoot 'selected-sources.json') -Raw | ConvertFrom-Json
if(($Status -ceq 'accepted-fixture') -ne ($SourceCommit -cmatch '^[a-f0-9]{40}$')){throw 'source/status identity'}
$m=@{
    schema='hc074-original-manager-grouped-message-v1';status=$Status;base_commit=$selected.base_commit
    source_commit=$(if($Status -ceq 'accepted-fixture'){$SourceCommit}else{$null})
    source_files=$selected.source_files;build_inputs=$selected.build_inputs
    admission=$false;original_start=$false;writer_cessation=$false;server_install=$false;supervisor_changed=$false;product_workload=$false;negotiated_bus_delivery=$false;positive_production_join=$false
    host_fixture_cohort_pass=($Status -ceq 'accepted-fixture')
    images=(Get-Content (Join-Path $PSScriptRoot 'Candidate-images.json') -Raw | ConvertFrom-Json)
    qualification_sha256=(Get-FileHash -LiteralPath (Join-Path $root 'docs/testing/performance/0.74/qualification-manifest.toml') -Algorithm SHA256).Hash.ToLowerInvariant()
}
$m.files=@(Get-ChildItem -LiteralPath $PSScriptRoot -Recurse -File | Where-Object {$_.Name -cne 'manifest.json'} | Sort-Object FullName | ForEach-Object {@{path=[IO.Path]::GetRelativePath($PSScriptRoot,$_.FullName).Replace('\','/');sha256=(Get-FileHash -LiteralPath $_.FullName -Algorithm SHA256).Hash.ToLowerInvariant()}})
$m | ConvertTo-Json -Depth 10 | Set-Content -Encoding utf8NoBOM (Join-Path $PSScriptRoot 'manifest.json')
"HC074-GROUPED-CAPTURE status=$Status admission=false"
