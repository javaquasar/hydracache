$ErrorActionPreference='Stop'
$root=(Resolve-Path (Join-Path $PSScriptRoot '../../../../../..')).Path
$base='959456253a74a6c0a2e9dfee9a7615f447af7448'
function HashText([string]$Text) {[Convert]::ToHexString([Security.Cryptography.SHA256]::HashData([Text.Encoding]::UTF8.GetBytes($Text))).ToLowerInvariant()}
function GitText([string]$Path) {
    $s=[Diagnostics.ProcessStartInfo]::new('git');$s.WorkingDirectory=$root;$s.UseShellExecute=$false;$s.RedirectStandardOutput=$true;$s.StandardOutputEncoding=[Text.UTF8Encoding]::new($false)
    $s.ArgumentList.Add('show');$s.ArgumentList.Add("${base}:$Path")
    $p=[Diagnostics.Process]::Start($s);$v=$p.StandardOutput.ReadToEnd();$p.WaitForExit();if($p.ExitCode -ne 0){throw 'base input'};$v.Replace("`r`n","`n")
}
$sources=@(Get-Content (Join-Path $PSScriptRoot 'selected-sources.json') -Raw | ConvertFrom-Json | ForEach-Object {
    $capsule=Join-Path $PSScriptRoot ('source-capsule/'+$_.path.Replace('/','__')+'.source')
    if((Get-FileHash -LiteralPath $capsule -Algorithm SHA256).Hash.ToLowerInvariant() -cne $_.sha256){throw 'selected capsule drift'}
    @{path=$_.path;raw_sha256=$_.sha256;lf_sha256=(HashText ([IO.File]::ReadAllText($capsule).Replace("`r`n","`n")))}
})
$inputs=@(git -C $root ls-tree -r --name-only $base -- tools/long-run-supervisor-074/src | Where-Object {$_ -cmatch '^tools/long-run-supervisor-074/src/[^/]+\.rs$'})
$inputs+=@($sources.path | Where-Object {$_ -cmatch '^tools/long-run-supervisor-074/src/[^/]+\.rs$'})
$inputs+=@('Cargo.toml','Cargo.lock','tools/long-run-supervisor-074/Cargo.toml')
$bindings=@(foreach($path in ($inputs | Sort-Object -Unique)) {
    $changed=@($sources | Where-Object {$_.path -ceq $path})
    $hash=if($changed.Count -eq 1){$changed[0].lf_sha256}else{HashText (GitText $path)}
    @{path=$path;lf_sha256=$hash}
})
if($sources.Count -ne 14 -or $bindings.Count -ne 346){throw 'candidate inventory'}
$m=@{
    schema='hc074-original-manager-message-rejected-v1';base_commit=$base;source_commit=$null;status='withdrawn-deadline';source_files=$sources;build_inputs=$bindings
    admission=$false;original_start=$false;writer_cessation=$false;server_install=$false;supervisor_changed=$false;product_workload=$false;negotiated_bus_delivery=$false;positive_production_join=$false;host_fixture_cohort_pass=$false;candidate_promoted=$false
    images=@(Get-Content (Join-Path $PSScriptRoot 'selected-images.json') -Raw | ConvertFrom-Json)
    attribution_image=(Get-Content (Join-Path $PSScriptRoot 'attribution-image.json') -Raw | ConvertFrom-Json)
    qualification_sha256=(Get-FileHash -LiteralPath (Join-Path $root 'docs/testing/performance/0.74/qualification-manifest.toml') -Algorithm SHA256).Hash.ToLowerInvariant()
}
$m.files=@(Get-ChildItem -LiteralPath $PSScriptRoot -Recurse -File | Where-Object {$_.Name -cne 'manifest.json'} | Sort-Object FullName | ForEach-Object {@{path=[IO.Path]::GetRelativePath($PSScriptRoot,$_.FullName).Replace('\','/');sha256=(Get-FileHash -LiteralPath $_.FullName -Algorithm SHA256).Hash.ToLowerInvariant()}})
$m | ConvertTo-Json -Depth 10 | Set-Content -Encoding utf8NoBOM (Join-Path $PSScriptRoot 'manifest.json')
'HC074-ADDRESSED-REJECTED-CAPTURE sources=14 inputs=346 promoted=false'
