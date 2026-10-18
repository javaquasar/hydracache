param([switch]$SelfTest)
$ErrorActionPreference='Stop'
function Fail([bool]$Test,[string]$Reason) {if (!$Test) {throw $Reason}}
function Hash([string]$Path) {(Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant()}
function TextHash([string]$Text) {[Convert]::ToHexString([Security.Cryptography.SHA256]::HashData([Text.Encoding]::UTF8.GetBytes($Text))).ToLowerInvariant()}
function Parse([string]$Raw) {
    $rows=@()
    $controls=@()
    $compiler=''
    $names=@('kernel-worker','worker-policy','worker-source','active-authority','kernel-context','local-context')
    $counts=@(32,64,128,128,64,32)
    foreach ($line in ($Raw -split "\r?\n")) {
        $prefix='test diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::tests::fence_preflight_tests::attempt_tests::collector_tests::observed_reservation_tests::collector_reserved_tests::progress_tests::start_consent_tests::validation_frame_tests::starting_tests::sink_tests::start_material_tests::pipe_transport_tests::safety_alarm_tests::leaf_cost_tests::component_cost_tests::live_walk_tests::original_live_worker_walk_attribution ... '
        if ($line.StartsWith($prefix,[StringComparison]::Ordinal)) {$line=$line.Substring($prefix.Length)}
        if ($line -cmatch '^HC074-LIVE-WALK-HOST compiler=(194|188) begin=true$') {$compiler=$Matches[1]}
        if ($line -cmatch '^HC074-LIVE-WALK sample=([0-2]) kind=([a-z-]+) stats=Stats \{ calls: ([0-9]+), inclusive_ns: ([0-9]+), outside_children_ns: ([0-9]+) \}$') {
            Fail ($compiler -cne '') 'row without compiler'
            $sample=[int]$Matches[1];$kind=$Matches[2];$calls=[long]$Matches[3];$inclusive=[long]$Matches[4];$outside=[long]$Matches[5]
            $index=[Array]::IndexOf($names,$kind)
            Fail ($index -ge 0 -and $calls -eq $counts[$index] -and $outside -le $inclusive) 'span counters'
            $rows+=@{compiler=$compiler;sample=$sample;kind=$kind;calls=$calls;inclusive_ns=$inclusive;outside_children_ns=$outside}
        }
        if ($line -cmatch '^HC074-LIVE-WALK-CONTROL sample=([0-2]) iterations=32 original=true unchanged=true reservation=false schedule_changed=false admission=false$') {
            $controls+="$compiler/$($Matches[1])"
        }
    }
    Fail ($rows.Count -eq 36 -and $controls.Count -eq 6) 'row/control inventory'
    foreach ($c in @('194','188')) {
        Fail ($Raw.Contains("HC074-LIVE-WALK-HOST compiler=$c completed=true admission=false")) 'compiler not complete'
        foreach ($sample in 0..2) {
            Fail (@($controls | Where-Object {$_ -ceq "$c/$sample"}).Count -eq 1) 'control membership'
            $batch=@($rows | Where-Object {$_.compiler -ceq $c -and $_.sample -eq $sample})
            Fail ($batch.Count -eq 6) 'batch inventory'
            for ($i=0;$i -lt 6;$i++) {Fail ($batch[$i].kind -ceq $names[$i]) 'kind/order'}
            Fail (($batch[0].inclusive_ns - $batch[0].outside_children_ns) -eq ($batch[1].inclusive_ns+$batch[4].inclusive_ns+$batch[5].inclusive_ns)) 'worker nesting'
            Fail (($batch[1].inclusive_ns - $batch[1].outside_children_ns) -eq ($batch[2].inclusive_ns+$batch[3].inclusive_ns)) 'policy nesting'
            foreach ($i in @(2,3,4,5)) {Fail ($batch[$i].inclusive_ns -eq $batch[$i].outside_children_ns) 'leaf nesting'}
        }
    }
    Fail ($Raw.Contains('HC074-LIVE-WALK-HOST completed=true supervisor_unchanged=true no_fixture_executable=true admission=false')) 'host not complete'
    Fail (($Raw -split 'test result: ok\. 1 passed; 0 failed; 0 ignored; 0 measured;').Count -eq 3) 'host test outcomes'
    return ,$rows
}
$repo=(Resolve-Path (Join-Path $PSScriptRoot '../../../../../..')).Path
$manifest=Get-Content (Join-Path $PSScriptRoot 'manifest.json') -Raw | ConvertFrom-Json -AsHashtable
Fail ($manifest.status -ceq 'accepted-diagnostic') 'not accepted diagnostic'
foreach ($flag in @('start_admitted','measurement_admitted','installation_performed','supervisor_changed')) {Fail ($manifest[$flag] -ceq $false) 'admission flag'}
$files=@(Get-ChildItem -LiteralPath $PSScriptRoot -Recurse -Force -File | Where-Object {$_.Name -cne 'manifest.json'} | ForEach-Object {[IO.Path]::GetRelativePath($PSScriptRoot,$_.FullName).Replace('\','/')} | Sort-Object)
Fail (($files -join "`n") -ceq (($manifest.files.path | Sort-Object) -join "`n")) 'closed file inventory'
foreach ($file in $manifest.files) {
    $path=Join-Path $PSScriptRoot $file.path
    Fail ((Hash $path) -ceq $file.sha256 -and (Get-Item -LiteralPath $path).Length -eq $file.bytes) "packet hash: $($file.path)"
}
$binding=Get-Content (Join-Path $PSScriptRoot 'selected-sources.json') -Raw | ConvertFrom-Json
Fail ($binding.source_commit -ceq $manifest.source_commit -and $binding.source_commit -cmatch '^[a-f0-9]{40}$') 'source identity'
Fail ($binding.baseline_commit -ceq 'b2adb5b73a2f66fc690637260c100b5c8489aa35' -and $binding.preregistration_commit -ceq 'd5cb0b0e') 'baseline/preregistration'
Fail ($binding.sources.Count -eq 14 -and $binding.build_inputs.Count -eq 362 -and $binding.embedded_inputs.Count -eq 12 -and $binding.literal_embed_calls -eq 27) 'source inventory'
$changed=@(& git -C $repo diff --name-only d5cb0b0e $binding.source_commit)
Fail ($LASTEXITCODE -eq 0 -and (($changed | Sort-Object) -join "`n") -ceq (($binding.sources.path | Sort-Object) -join "`n")) 'exact selected source diff'
$rust=@(& git -C $repo ls-tree -r --name-only $binding.source_commit -- tools/long-run-supervisor-074/src | Where-Object {$_ -clike '*.rs'})
Fail ($LASTEXITCODE -eq 0 -and $rust.Count -eq 351) 'committed Rust inventory'
$expected=@(@($rust)+@('Cargo.toml','Cargo.lock','tools/long-run-supervisor-074/Cargo.toml')+@($binding.embedded_inputs.path) | Sort-Object -Unique)
Fail ((($expected | Sort-Object) -join "`n") -ceq (($binding.build_inputs.path | Sort-Object) -join "`n")) 'exact build input inventory'
$unique=@{}
foreach ($input in $binding.build_inputs) {
    Fail (!$unique.ContainsKey($input.path)) 'duplicate build input'
    $unique[$input.path]=$input.lf_sha256
}
foreach ($source in $binding.sources) {
    $path=Join-Path $PSScriptRoot ('source-capsule/'+$source.path.Replace('/','__')+'.source')
    Fail ((Hash $path) -ceq $source.raw_sha256 -and (TextHash ([IO.File]::ReadAllText($path).Replace("`r`n","`n"))) -ceq $source.lf_sha256) 'capsule hash'
}
$cache=@{}
function GitText([string]$Path) {
    if ($cache.ContainsKey($Path)) {return $cache[$Path]}
    $psi=[Diagnostics.ProcessStartInfo]::new('git')
    $psi.WorkingDirectory=$repo;$psi.RedirectStandardOutput=$true;$psi.RedirectStandardError=$true;$psi.UseShellExecute=$false
    $psi.ArgumentList.Add('show');$psi.ArgumentList.Add("$($binding.source_commit):$Path")
    $process=[Diagnostics.Process]::Start($psi)
    $raw=$process.StandardOutput.ReadToEnd()
    $errorText=$process.StandardError.ReadToEnd()
    $process.WaitForExit()
    Fail ($process.ExitCode -eq 0) "git source: $Path $errorText"
    $cache[$Path]=$raw.Replace("`r`n","`n")
    return $cache[$Path]
}
foreach ($input in $binding.build_inputs) {Fail ((TextHash (GitText $input.path)) -ceq $input.lf_sha256) "committed input: $($input.path)"}
foreach ($source in $binding.sources) {Fail ((TextHash (GitText $source.path)) -ceq $source.lf_sha256) "committed capsule: $($source.path)"}
$literalCalls=0
$embeddedPaths=@(foreach ($path in $rust) {
    $raw=GitText $path
    $matches=[regex]::Matches($raw,'\binclude_(?:str|bytes)!\s*\(\s*"([^"]+)"\s*,?\s*\)')
    Fail ($matches.Count -eq [regex]::Matches($raw,'\binclude_(?:str|bytes)!\s*\(').Count) 'nonliteral embedded input'
    $literalCalls+=$matches.Count
    $parent=[IO.Path]::GetDirectoryName((Join-Path $repo $path))
    foreach ($m in $matches) {[IO.Path]::GetRelativePath($repo,[IO.Path]::GetFullPath((Join-Path $parent $m.Groups[1].Value))).Replace('\','/')}
})
$embeddedPaths=@($embeddedPaths | Sort-Object -Unique)
Fail ($literalCalls -eq 27 -and ($embeddedPaths -join "`n") -ceq (($binding.embedded_inputs.path | Sort-Object) -join "`n")) 'embedded closure'
foreach ($input in $binding.embedded_inputs) {
    $path=Join-Path $PSScriptRoot ('embedded-capsule/'+$input.path.Replace('/','__')+'.source')
    Fail ((Hash $path) -ceq $input.raw_sha256 -and (TextHash ([IO.File]::ReadAllText($path).Replace("`r`n","`n"))) -ceq $input.lf_sha256 -and (TextHash (GitText $input.path)) -ceq $input.lf_sha256) 'embedded raw/source identity'
}
Fail ($manifest.qualification_manifest_sha256 -ceq '11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc') 'qualification pin'
Fail ((Hash (Join-Path $repo 'docs/testing/performance/0.74/qualification-manifest.toml')) -ceq $manifest.qualification_manifest_sha256) 'qualification changed'
$images=Get-Content (Join-Path $PSScriptRoot 'images.json') -Raw | ConvertFrom-Json
Fail ($images.Count -eq 2) 'image inventory'
$hostRaw=Get-Content (Join-Path $PSScriptRoot 'host.log') -Raw
foreach ($script in @(@('host-guard.sh','guard.sh'),@('host-run.sh','run.sh'))) {
    Fail ($hostRaw.Contains("$(Hash (Join-Path $PSScriptRoot $script[0]))  ./$($script[1])")) 'executed host script hash'
}
foreach ($name in @('host-preflight.log','host-postflight.log')) {
    $guard=Get-Content (Join-Path $PSScriptRoot $name) -Raw
    Fail ($guard.Trim() -ceq 'HC074-ORIGINAL-MANAGER-HOST preflight=ok supervisor_pid=8839 restarts=0') 'independent host guard'
}
foreach ($compiler in @('194','188')) {
    $image=@($images | Where-Object {$_.compiler -ceq $compiler})
    Fail ($image.Count -eq 1 -and $image[0].sha256 -cmatch '^[a-f0-9]{64}$') 'image identity'
    Fail ($hostRaw.Contains("$($image[0].sha256)  ./tests-$compiler")) 'host image hash'
    $json=Get-Content (Join-Path $PSScriptRoot "build-$compiler.jsonl") | ForEach-Object {$_ | ConvertFrom-Json}
    $artifact=@($json | Where-Object {$_.reason -ceq 'compiler-artifact' -and $_.target.name -ceq 'hydracache_long_run_supervisor_074' -and $_.profile.test -and $_.executable})
    Fail ($artifact.Count -eq 1 -and $artifact[0].executable -ceq $image[0].image) 'built image path'
    Fail (@($json | Where-Object {$_.reason -ceq 'build-finished' -and $_.success -ceq $true}).Count -eq 1) 'build outcome'
}
$rows=Parse $hostRaw
$observed=Get-Content (Join-Path $PSScriptRoot 'observations.json') -Raw | ConvertFrom-Json
Fail ($observed.rows.Count -eq 36) 'observation count'
for ($i=0;$i -lt 36;$i++) {
    foreach ($key in @('compiler','sample','kind','calls','inclusive_ns','outside_children_ns')) {Fail ($rows[$i][$key] -ceq $observed.rows[$i].$key) 'observation mismatch'}
}
if ($SelfTest) {
    $mutations=@(
        $hostRaw.Replace('calls: 128,','calls: 127,'),
        $hostRaw.Replace('unchanged=true','unchanged=false'),
        $hostRaw.Replace('kind=local-context','kind=worker-policy'),
        $hostRaw.Replace('supervisor_unchanged=true','supervisor_unchanged=false'),
        $hostRaw.Replace('compiler=188 begin=true','compiler=194 begin=true'),
        $hostRaw.Replace('schedule_changed=false','schedule_changed=true')
    )
    foreach ($mutation in $mutations) {
        $refused=$false
        try {$null=Parse $mutation} catch {$refused=$true}
        Fail $refused 'negative control accepted'
    }
}
"HC074-LIVE-WALK-VERIFY rows=36 sources=14 inputs=362 embedded=12 negative_controls=$(if($SelfTest){6}else{0}) admission=false"
