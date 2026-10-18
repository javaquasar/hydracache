param([switch]$SelfTest)
$ErrorActionPreference='Stop'
$repo=(Resolve-Path (Join-Path $PSScriptRoot '../../../../../..')).Path
function Require([bool]$Value,[string]$Why) {if (!$Value) {throw $Why}}
function Hash([string]$Path) {(Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant()}
function TextHash([string]$Value) {[Convert]::ToHexString([Security.Cryptography.SHA256]::HashData([Text.Encoding]::UTF8.GetBytes($Value))).ToLowerInvariant()}
function Read([string]$Path) {[IO.File]::ReadAllText((Join-Path $PSScriptRoot $Path))}
function LF([string]$Value) {$Value.Replace("`r`n","`n")}
$m=Get-Content -LiteralPath (Join-Path $PSScriptRoot 'manifest.json') -Raw | ConvertFrom-Json -AsHashtable
Require ($m.schema -ceq 'hc074-boot-buffer-plain-v1' -and $m.source_commit -cmatch '^[a-f0-9]{40}$') 'manifest identity'
foreach ($flag in @('start_admitted','measurement_admitted','installation_performed','supervisor_changed')) {Require ($m[$flag] -ceq $false) "claim $flag"}
$files=@(Get-ChildItem -LiteralPath $PSScriptRoot -Recurse -Force -File | Where-Object {$_.Name -cne 'manifest.json'} | ForEach-Object {[IO.Path]::GetRelativePath($PSScriptRoot,$_.FullName).Replace('\','/')} | Sort-Object)
Require (($files -join ',') -ceq (($m.files.path | Sort-Object) -join ',')) 'closed inventory'
foreach ($file in $m.files) {Require ((Hash (Join-Path $PSScriptRoot $file.path)) -ceq $file.sha256 -and (Get-Item -LiteralPath (Join-Path $PSScriptRoot $file.path)).Length -eq $file.bytes) "file hash $($file.path)"}
$binding=Get-Content -LiteralPath (Join-Path $PSScriptRoot 'source-binding.json') -Raw | ConvertFrom-Json -AsHashtable
Require ($binding.source_commit -ceq $m.source_commit -and $binding.rust_sources -eq 356 -and $binding.selected_inputs -eq 367 -and $binding.literal_embed_calls -eq 27 -and $binding.complete_compiler_attestation -ceq $false) 'source census'
$cache=@{}
function GitText([string]$Path,[string]$Ref=$m.source_commit) {
    $key="$Ref`:$Path"
    if ($cache.ContainsKey($key)) {return $cache[$key]}
    $psi=[Diagnostics.ProcessStartInfo]::new('git')
    $psi.WorkingDirectory=$repo;$psi.UseShellExecute=$false;$psi.RedirectStandardOutput=$true;$psi.RedirectStandardError=$true
    $psi.ArgumentList.Add('show');$psi.ArgumentList.Add($key)
    $p=[Diagnostics.Process]::Start($psi)
    $text=$p.StandardOutput.ReadToEnd();$errorText=$p.StandardError.ReadToEnd();$p.WaitForExit()
    Require ($p.ExitCode -eq 0) "Git input $Path $errorText"
    $cache[$key]=LF $text
    return $cache[$key]
}
$changed=@(& git -C $repo diff --name-only $binding.preregistration_commit $m.source_commit)
Require ($LASTEXITCODE -eq 0 -and $changed.Count -eq 14 -and ($changed -join ',') -ceq (($binding.sources.path | Sort-Object) -join ',')) 'exact source change set'
foreach ($source in $binding.sources) {Require ((LF (Read $source.capsule)) -ceq (GitText $source.path)) "source capsule $($source.path)"}
$inputs=Get-Content -LiteralPath (Join-Path $PSScriptRoot 'receipts/Candidate-194-inputs.json') -Raw | ConvertFrom-Json -AsHashtable
& (Join-Path $PSScriptRoot 'compare-inputs.ps1') -Directory (Join-Path $PSScriptRoot 'receipts')
foreach ($input in $inputs) {Require ((TextHash (GitText $input.path)) -ceq $input.lf_sha256) "committed input $($input.path)"}
$rust=@(& git -C $repo ls-tree -r --name-only $m.source_commit -- tools/long-run-supervisor-074/src | Where-Object {$_.EndsWith('.rs',[StringComparison]::Ordinal)})
Require ($LASTEXITCODE -eq 0 -and $rust.Count -eq 356) 'committed Rust inventory'
$literal=0
$embedded=@(foreach ($path in $rust) {
    $text=GitText $path
    $matches=[regex]::Matches($text,'\binclude_(?:str|bytes)!\s*\(\s*"([^"]+)"\s*,?\s*\)')
    Require ($matches.Count -eq [regex]::Matches($text,'\binclude_(?:str|bytes)!\s*\(').Count) 'nonliteral embed'
    $literal+=$matches.Count
    foreach ($match in $matches) {[IO.Path]::GetRelativePath($repo,[IO.Path]::GetFullPath((Join-Path ([IO.Path]::GetDirectoryName((Join-Path $repo $path))) $match.Groups[1].Value))).Replace('\','/')}
})
$embedded=@($embedded | Sort-Object -Unique)
Require ($literal -eq 27 -and $embedded.Count -eq 12 -and ($embedded -join ',') -ceq (($binding.embedded.path | Sort-Object) -join ',')) 'embedded inventory'
$expected=@($rust+@('Cargo.toml','Cargo.lock','tools/long-run-supervisor-074/Cargo.toml')+$embedded | Sort-Object -Unique)
Require (($expected -join ',') -ceq (($inputs.path | Sort-Object) -join ',')) 'selected input inventory'
foreach ($embed in $binding.embedded) {
    $entry=@($inputs | Where-Object {$_.path -ceq $embed.path})
    Require ($entry.Count -eq 1 -and (Hash (Join-Path $PSScriptRoot $embed.capsule)) -ceq $entry[0].raw_sha256 -and (LF (Read $embed.capsule)) -ceq (GitText $embed.path)) 'embedded capsule'
}
$origin='tools/long-run-supervisor-074/src/diagnostic_kernel_origin.rs'
$candidate=LF (Read 'receipts/Candidate-194-origin.source')
$baseline=LF (Read 'receipts/Baseline-194-origin.source')
Require ($candidate -ceq (GitText $origin)) 'candidate origin identity'
$old=GitText $origin '3b8e73feec8412e4090bf05575f06a05901e1010'
$begin=$candidate.IndexOf('fn boot(root:');$end=$candidate.IndexOf('fn boot_read(',$begin)
$oldBegin=$old.IndexOf('fn boot(root:');$oldEnd=$old.IndexOf('fn clock(',$oldBegin)
$newFunction=$candidate.Substring($begin,$end-$begin);$oldFunction=$old.Substring($oldBegin,$oldEnd-$oldBegin)
Require ($candidate.Replace($newFunction,$oldFunction) -ceq $baseline) 'exact old boot overlay'
$uploads=Get-Content -LiteralPath (Join-Path $PSScriptRoot 'receipts/upload-inputs.json') -Raw | ConvertFrom-Json -AsHashtable
Require ($uploads.Count -eq 15 -and @($uploads.path | Select-Object -Unique).Count -eq 15) 'upload inventory'
foreach ($compiler in @('194','188')) {
    foreach ($role in @('Baseline','Candidate')) {
        $tag="$role-$compiler"
        $entry=Get-Content -LiteralPath (Join-Path $PSScriptRoot "receipts/$tag.json") -Raw | ConvertFrom-Json -AsHashtable
        Require ($entry.compiler -ceq $compiler -and $entry.role -ceq $role -and $entry.inputs -eq 367) 'image metadata'
        Require ((Hash (Join-Path $PSScriptRoot "receipts/$tag-origin.source")) -ceq $entry.origin_raw_sha256) 'image source hash'
        $image=@(Get-Content -LiteralPath (Join-Path $PSScriptRoot "receipts/$tag-build.jsonl") | ForEach-Object {$_ | ConvertFrom-Json} | Where-Object {$_.reason -ceq 'compiler-artifact' -and $_.target.name -ceq 'hydracache_long_run_supervisor_074' -and $_.profile.test -ceq $true -and $_.executable})
        Require ($image.Count -eq 1 -and $image[0].executable -ceq $entry.image) 'compiler artifact'
        $uploaded=@($uploads | Where-Object {$_.path -ceq "tests-$tag"})
        Require ($uploaded.Count -eq 1 -and $uploaded[0].sha256 -ceq $entry.sha256) 'uploaded image identity'
    }
}
$sum=(($uploads | ForEach-Object {"$($_.sha256)  $($_.path)"}) -join "`n")+"`n"
Require ((Read 'receipts/SHA256SUMS') -ceq $sum) 'uploaded checksum file'
foreach ($pair in @(@('guard.sh','host-guard.sh'),@('run.sh','host-run.sh'))) {
    $entry=@($uploads | Where-Object {$_.path -ceq $pair[0]})
    Require ($entry.Count -eq 1 -and $entry[0].sha256 -ceq (Hash (Join-Path $PSScriptRoot $pair[1]))) 'executed script identity'
}
$bundle=@($uploads | Where-Object {$_.path -ceq 'bundle/timing-controls-074'})
Require ($bundle.Count -eq 1 -and $bundle[0].sha256 -ceq 'bda1c094f3de7c6d42ec45c757838c7a41a8ff1d63a025c661af84455169a101') 'nonexecuted bundle executable identity'
$exits=Get-Content -LiteralPath (Join-Path $PSScriptRoot 'receipts/host-exits.json') -Raw | ConvertFrom-Json -AsHashtable
Require ($exits.run_exit -eq 0 -and $exits.post_exit -eq 0 -and $exits.retry -ceq $false) 'host exits'
foreach ($log in @('host-preflight.log','host-postflight.log')) {
    $raw=Read "receipts/$log"
    Require ($raw.Contains('preflight=ok supervisor_pid=8839 restarts=0')) 'stable host pre/post guard'
    foreach ($entry in $uploads) {Require ($raw.Contains("$($entry.path): OK")) 'host checksum verification'}
}
Require ((Read 'receipts/host-postflight.log').Contains('HC074-PLAIN-INDEPENDENT-POSTFLIGHT complete=true supervisor_unchanged=true no_fixture=true')) 'independent postflight'
. (Join-Path $PSScriptRoot 'rows.ps1')
$parsed=Read-HostRows (Read 'receipts/host.log')
$obs=Get-Content -LiteralPath (Join-Path $PSScriptRoot 'observations.json') -Raw | ConvertFrom-Json -AsHashtable
Require ($obs.product_claim -ceq $false -and $obs.raw.rows.Count -eq 20 -and $obs.raw.ages.Count -eq 12 -and $obs.raw.refusals -eq 40 -and $obs.raw.transfers -eq 6 -and $obs.raw.fixed_refusals -eq 2 -and $obs.raw.healthy_rounds -eq 12) 'observed inventory'
foreach ($kind in @('rows','ages')) {
    for ($i=0; $i -lt $parsed[$kind].Count; $i++) {
        foreach ($key in $parsed[$kind][$i].Keys) {Require ($parsed[$kind][$i][$key] -ceq $obs.raw[$kind][$i][$key]) 'raw observation decode'}
    }
}
$nonRegression=$true
foreach ($compiler in @('194','188')) {
    $stats=@($obs.statistics | Where-Object {$_.compiler -ceq $compiler})
    Require ($stats.Count -eq 1 -and $stats[0].pairs.Count -eq 5 -and $stats[0].product_claim -ceq $false) 'statistical inventory'
    $logs=@()
    foreach ($pair in 0..4) {
        $a=@($parsed.rows | Where-Object {$_.compiler -ceq $compiler -and $_.pair -eq $pair -and $_.role -ceq 'Baseline'})[0]
        $b=@($parsed.rows | Where-Object {$_.compiler -ceq $compiler -and $_.pair -eq $pair -and $_.role -ceq 'Candidate'})[0]
        $ratio=[double]$b.wall_ns/[double]$a.wall_ns;$logs+=[Math]::Log($ratio)
        Require ([Math]::Abs($stats[0].pairs[$pair].ratio-$ratio) -lt 1e-12 -and $stats[0].pairs[$pair].baseline_ns_per_validation -eq ([double]$a.wall_ns/128) -and $stats[0].pairs[$pair].candidate_ns_per_validation -eq ([double]$b.wall_ns/128)) 'pair estimate or units'
    }
    $mean=($logs[0]+$logs[1]+$logs[2]+$logs[3]+$logs[4])/5
    $sum=0.0;foreach ($log in $logs) {$sum+=($log-$mean)*($log-$mean)}
    $half=2.7764451051977987*[Math]::Sqrt($sum/20)
    $low=[Math]::Exp($mean-$half);$high=[Math]::Exp($mean+$half)
    Require ([Math]::Abs($stats[0].ci95_lower-$low) -lt 1e-12 -and [Math]::Abs($stats[0].ci95_upper-$high) -lt 1e-12 -and [Math]::Abs($stats[0].geometric_mean_ratio-[Math]::Exp($mean)) -lt 1e-12) 'independent interval'
    Require ($stats[0].lower_cost_confirmed -ceq ($high -le 0.99) -and $stats[0].non_regression -ceq ($high -le 1.03)) 'frozen statistical criteria'
    $nonRegression=$nonRegression -and ($high -le 1.03)
}
Require ($m.preparation_accepted -ceq $nonRegression -and $obs.preparation_accepted -ceq $nonRegression) 'preparation decision'
Require ($m.status -ceq $(if ($nonRegression) {'accepted-preparation-only'} else {'inconclusive-preparation'})) 'preparation status'
Require ((Read 'receipts/checks/contracts.log').Contains('test result: ok. 251 passed; 0 failed;')) 'all source contracts'
Require ((Read 'receipts/checks/windows-tests.log').Contains('test result: ok. 44 passed; 0 failed;')) 'portable controls'
foreach ($gate in @(
    @('registry','gated-test-check: OK (184 discovered gates, all registered)'),
    @('docs-final','doc-check: OK (releases.toml is consistent)'),
    @('performance-contract','performance-contract-check 0.74: OK (local, non-promotable)')
)) {Require ((Read "receipts/checks/$($gate[0]).log").Contains($gate[1])) "local gate $($gate[0])"}
foreach ($name in @('windows-check','windows-clippy','gnu-194-check','gnu-194-clippy','gnu-188-check','gnu-188-clippy')) {Require ((Read "receipts/checks/$name.log").Contains('Finished `dev` profile')) 'check/Clippy'}
foreach ($compiler in @('194','188')) {
    foreach ($filter in @('boot_buffer_','kernel_walk::tests::','live_walk::tests::','validation_trace_tests::','manager_channel::','dispatch_exchange::','original_pipe_message_','worker_context_','diagnostic_kernel_admission::tests::','diagnostic_kernel_admission::origin::tests::')) {
        $name=$filter.Replace(':','').Replace('_','-')
        Require ((Read "receipts/checks/gnu-$compiler-$name.log") -cmatch 'test result: ok\. [1-9][0-9]* passed; 0 failed;') 'focused unit group'
    }
    Require ((Read "receipts/checks/gnu-$compiler-allocation.log").Contains('test result: ok. 4 passed; 0 failed;')) 'allocation controls'
}
Require ($m.qualification_manifest_sha256 -ceq '11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc' -and (Hash (Join-Path $repo 'docs/testing/performance/0.74/qualification-manifest.toml')) -ceq $m.qualification_manifest_sha256) 'frozen qualification'
if ($SelfTest) {& (Join-Path $PSScriptRoot 'parser-selftest.ps1')}
"HC074-PLAIN-VERIFY source=$($m.source_commit) inputs=367 differing=1 invocations=26 healthy_rounds=12 refusals=40 preparation_accepted=$nonRegression start=false"
