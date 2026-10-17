param([string]$Root = (Resolve-Path "$PSScriptRoot/../../../../../..").Path,
      [switch]$CapsulesOnly,
      [switch]$SelfTest)
$ErrorActionPreference = 'Stop'
$packet = $PSScriptRoot
$baseline = '8193e64202a5cff9fe769c0238534ff201151625'
function Require($ok, [string]$message) { if (-not $ok) { throw $message } }
function BlobHash([string]$value) {
    $body = [Text.Encoding]::UTF8.GetBytes($value)
    $prefix = [Text.Encoding]::UTF8.GetBytes("blob $($body.Length)`0")
    $sha = [Security.Cryptography.SHA1]::Create()
    try { ([BitConverter]::ToString($sha.ComputeHash($prefix + $body))).Replace('-', '').ToLowerInvariant() }
    finally { $sha.Dispose() }
}
function GitText([string]$path) {
    $lines = @(git -C $Root show "${baseline}:$path")
    Require ($LASTEXITCODE -eq 0) "missing baseline $path"
    ($lines -join "`n") + "`n"
}
function CheckCapsule([string]$name, [bool]$current, [int]$expected, [string]$Revision = '') {
    $raw = [IO.File]::ReadAllText((Join-Path $packet $name)).Replace("`r", '')
    $sections = [regex]::Split($raw, '(?m)(?=^diff --git )') | Where-Object { $_ }
    Require ($sections.Count -eq $expected) "capsule membership $name"
    $seen = @{}
    foreach ($section in $sections) {
        $lines = $section.TrimEnd("`n").Split("`n")
        Require ($lines[0] -match '^diff --git a/(.+) b/(.+)$') "capsule header $name"
        $path = $Matches[1]
        Require ($path -ceq $Matches[2]) "rename forbidden $name"
        Require (-not $seen.ContainsKey($path)) "duplicate path $path"
        $seen[$path] = $true
        $index = @($lines | Where-Object { $_ -match '^index [0-9a-f]{40}\.\.[0-9a-f]{40}' })
        Require ($index.Count -eq 1) "index $path"
        Require ($index[0] -match '^index ([0-9a-f]{40})\.\.([0-9a-f]{40})') "index parse"
        $beforeHash = $Matches[1]; $afterHash = $Matches[2]
        $source = if ($beforeHash -eq ('0' * 40)) { '' } else { GitText $path }
        if ($source) { Require ((BlobHash $source) -ceq $beforeHash) "baseline blob $path" }
        $old = if ($source) { $source.Substring(0, $source.Length - 1).Split("`n") } else { @() }
        $out = [Collections.Generic.List[string]]::new()
        $cursor = 0
        for ($i = 0; $i -lt $lines.Count; $i++) {
            if ($lines[$i] -notmatch '^@@ -(\d+)(?:,(\d+))? \+(\d+)(?:,(\d+))? @@') { continue }
            $start = [int]$Matches[1]
            $oldCount = if ($Matches[2]) { [int]$Matches[2] } else { 1 }
            $newStart = [int]$Matches[3]
            $newCount = if ($Matches[4]) { [int]$Matches[4] } else { 1 }
            $at = if ($start -eq 0) { 0 } else { $start - 1 }
            Require ($at -ge $cursor -and $at -le $old.Count) "hunk position $path"
            while ($cursor -lt $at) { $out.Add($old[$cursor]); $cursor++ }
            Require ($out.Count -eq [Math]::Max(0, $newStart - 1)) "new hunk position $path"
            $removed = 0; $added = 0
            while ($i + 1 -lt $lines.Count -and $lines[$i + 1] -notmatch '^@@ ') {
                $i++
                $line = $lines[$i]
                Require ($line.Length -gt 0) "empty patch line $path"
                $kind = $line[0]; $value = $line.Substring(1)
                Require ($kind -in @(' ', '-', '+')) "unsupported patch line $path"
                if ($kind -ne '+') {
                    Require ($cursor -lt $old.Count -and $old[$cursor] -ceq $value) "old hunk bytes $path"
                    $cursor++; $removed++
                }
                if ($kind -ne '-') { $out.Add($value); $added++ }
            }
            Require ($removed -eq $oldCount -and $added -eq $newCount) "hunk counts $path"
        }
        while ($cursor -lt $old.Count) { $out.Add($old[$cursor]); $cursor++ }
        $rebuilt = ($out -join "`n") + "`n"
        Require ((BlobHash $rebuilt) -ceq $afterHash) "rebuilt capsule blob $name $path"
        if ($Revision) {
            $committed = git -C $Root rev-parse "${Revision}:$path"
            Require ($LASTEXITCODE -eq 0 -and $committed -ceq $afterHash) "capsule committed cohort $path"
        }
        if ($current) {
            $actual = [IO.File]::ReadAllText((Join-Path $Root $path)).Replace("`r", '')
            Require ($actual -ceq $rebuilt) "current source differs $path"
        }
    }
    Write-Output "HC074-CAPSULE name=$name files=$expected exact=true current=$current"
}

function Healthy([string]$raw, [int]$samples) {
    Require ($raw.Contains('1 passed; 0 failed;')) 'healthy completion'
    Require (([regex]::Matches($raw, 'HC074-FIXED-GROUPED-CONTROL sample=')).Count -eq $samples) 'healthy controls'
    Require (([regex]::Matches($raw, 'HC074-FIXED-GROUPED-COST sample=')).Count -eq 2 * $samples) 'healthy spans'
    foreach ($sample in 0..($samples - 1)) {
        Require ($raw.Contains("HC074-FIXED-GROUPED-CONTROL sample=$sample original=true unchanged=true checkpoints=5 bounds_original=true reference=true guardian=false send=false admission=false")) 'original invariants'
        foreach ($phase in 'prepare', 'reference-validate') {
            $marker = "HC074-FIXED-GROUPED-COST sample=$sample phase=$phase report="
            # Split explicitly on the newline character, never a regex escape.
            $lines = @($raw.Split([char]10) | Where-Object { $_.Contains($marker) })
            Require ($lines.Count -eq 1 -and $lines[0].Contains('first_time: None, time_refusals: 0')) 'unrefused report'
            $stats = @([regex]::Matches($lines[0], 'Stats \{ wall_ns: (\d+), authentication_calls: (\d+), authentication_ns: (\d+), signature_calls: (\d+), signature_ns: (\d+), worker_policy_calls: (\d+), worker_policy_ns: (\d+) \}'))
            Require ($stats.Count -eq 8) 'trace stages'
            $active = if ($phase -ceq 'prepare') { 6 } else { 7 }
            for ($i = 0; $i -lt 8; $i++) {
                $g = $stats[$i].Groups
                $calls = if ($i -eq $active) { 34 } else { 0 }
                Require ([long]$g[2].Value -eq $calls -and [long]$g[4].Value -eq $calls) 'stage IPC counters'
                Require ([long]$g[6].Value -eq 0 -and [long]$g[7].Value -eq 0) 'unused worker counters'
            }
            $g = $stats[$active].Groups
            [pscustomobject]@{
                sample=$sample; phase=$phase; wall_ns=[long]$g[1].Value
                authentication_calls=[long]$g[2].Value; authentication_ns=[long]$g[3].Value
                signature_calls=[long]$g[4].Value; signature_ns=[long]$g[5].Value
            }
        }
    }
}
function Refusals([string]$raw) {
    Require ($raw.Contains('1 passed; 0 failed;')) 'refusal completion'
    $expected = @('trust/opened','trust/loaded','trust/bound','trust/message','trust/armed','bundle-content/loaded','bundle-inode/armed','output/armed','projection/armed','state/armed','writer/armed','message/armed','alarm/armed','expiry/armed')
    $rows = @([regex]::Matches($raw, 'HC074-FIXED-GROUPED-REFUSAL case=([a-z-]+) cut=([a-z]+) exact=true sticky=true held=true unchanged=true send=false'))
    Require ($rows.Count -eq 14 -and ([regex]::Matches($raw, 'HC074-FIXED-GROUPED-REFUSAL case=')).Count -eq 14) 'refusal membership'
    for ($i=0; $i -lt 14; $i++) {
        Require ("$($rows[$i].Groups[1].Value)/$($rows[$i].Groups[2].Value)" -ceq $expected[$i]) 'refusal order and cut'
    }
}
function Origin([string]$raw) {
    Require ($raw.Contains('1 passed; 0 failed;')) 'origin completion'
    Require (([regex]::Matches($raw,'HC074-FIXED-GROUPED-ORIGIN ')).Count -eq 2) 'origin membership'
    Require ($raw.Contains('HC074-FIXED-GROUPED-ORIGIN missing_fixed_install_refused=true fallback=false sticky=true held=true unchanged=true send=false')) 'fixed refusal without fallback'
    Require ($raw.Contains('HC074-FIXED-GROUPED-ORIGIN fixture_refused=true sticky=true held=true unchanged=true send=false')) 'fixture origin refused'
}
function CompilerChecks([string]$raw, [string]$lane) {
    Require ($raw.TrimEnd().EndsWith("HC074-FIXED-GROUPED-CHECKS lane=$lane completed=true")) 'compiler checks finished without a trailing shell error'
    Require ($raw -notmatch '(?m)^error(?:\[|:)|not found') 'compiler checks have no error'
    foreach ($count in 4,6,7,30) {
        Require ($raw.Contains("$count passed; 0 failed;")) 'focused test families'
    }
}
if ($SelfTest) {
    $healthy = Get-Content -Raw (Join-Path $packet 'screen-194.log')
    $faults = Get-Content -Raw (Join-Path $packet 'refusals-194.log')
    $origin = Get-Content -Raw (Join-Path $packet 'origin-194.log')
    $checks = Get-Content -Raw (Join-Path $packet 'checks-194.log')
    $null = Healthy $healthy 1
    Refusals $faults
    Origin $origin
    CompilerChecks $checks '194'
    $tests = @(
        { $null = Healthy ($healthy.Replace('checkpoints=5','checkpoints=4')) 1 },
        { $null = Healthy ($healthy.Replace('authentication_calls: 34','authentication_calls: 33')) 1 },
        { $null = Healthy ($healthy.Replace('bounds_original=true','bounds_original=false')) 1 },
        { $null = Healthy ($healthy.Replace('first_time: None','first_time: Some(Time)')) 1 },
        { $null = Healthy ($healthy + $healthy) 1 },
        { Refusals ($faults.Replace('cut=loaded','cut=armed')) },
        { Refusals ($faults.Replace('held=true','held=false')) },
        { Refusals ($faults.Replace('case=expiry','case=omitted')) },
        { Origin ($origin.Replace('fallback=false','fallback=true')) },
        { Origin ($origin.Replace('fixture_refused=true','fixture_refused=false')) },
        { CompilerChecks ($checks + 'all-features: not found') '194' },
        { CompilerChecks ('error: synthetic compiler failure' + [char]10 + $checks) '194' },
        { CompilerChecks (Get-Content -Raw (Join-Path $packet 'checks-overlap-194.log')) '194' }
    )
    foreach ($test in $tests) {
        $refused = $false
        try { & $test } catch { $refused = $true }
        Require $refused 'verifier mutant accepted'
    }
    Write-Output 'HC074-FIXED-GROUPED-VERIFIER accepted=4 refused=13 passed=true'
    return
}
CheckCapsule 'implementation.patch' $false 10 '7ddcd6f3bd6507e945048147971a5cc4bca15be6'
$baseline = '7ddcd6f3bd6507e945048147971a5cc4bca15be6'
CheckCapsule 'origin-harness-fix.patch' $true 1
$baseline = '8193e64202a5cff9fe769c0238534ff201151625'
if ($CapsulesOnly) { return }
$manifest = Get-Content -Raw (Join-Path $packet 'manifest.json') | ConvertFrom-Json
Require ($manifest.baseline -ceq $baseline) 'baseline'
Require ($manifest.implementation_source -ceq '7ddcd6f3bd6507e945048147971a5cc4bca15be6') 'original implementation cohort'
Require ($manifest.origin_source -ceq '1d0c9e4eacb54e6803ccc24e57ef1ae9daaf16ff') 'corrected origin cohort'
Require ($manifest.healthy_controls -eq 7 -and $manifest.persistent_refusals -eq 28 -and $manifest.origin_refusals -eq 4) 'exact result counts'
Require ($manifest.production_preparation_added -eq $true -and $manifest.fixture_runs_only -eq $true) 'production and test scope'
foreach ($flag in 'deadlines_changed','old_validator_bodies_changed','guardian','send','start','admission','ship') {
    Require ($manifest.$flag -ceq $false) "scope $flag"
}
Require ($manifest.sources.Count -eq 10 -and $manifest.binaries.Count -eq 2) 'source and compiler membership'
$capsule = Get-Content -Raw (Join-Path $packet 'implementation.patch')
$capsulePaths = @([regex]::Matches($capsule, '(?m)^diff --git a/([^\r\n]+) b/') | ForEach-Object { $_.Groups[1].Value } | Sort-Object)
Require (($capsulePaths -join ',') -ceq (@($manifest.sources.path | Sort-Object) -join ',')) 'capsule and manifest membership'
Require ($manifest.origin_sources.Count -eq 1 -and $manifest.origin_binaries.Count -eq 2) 'origin correction membership'
Require ($manifest.origin_sources[0].path -ceq 'tools/long-run-supervisor-074/src/diagnostic_fixed_grouped_preparation_join_tests.rs') 'only origin test changed'
Require ((@($manifest.binaries.lane | Sort-Object) -join ',') -ceq '188,194') 'exact compiler set'
Require ((@($manifest.origin_binaries.lane | Sort-Object) -join ',') -ceq '188,194') 'exact corrected compiler set'
foreach ($source in $manifest.sources) {
    $actual = [IO.File]::ReadAllText((Join-Path $Root $source.path)).Replace([string][char]13, '')
    $expected = if ($source.path -ceq $manifest.origin_sources[0].path) { $manifest.origin_sources[0].git_blob } else { $source.git_blob }
    Require ((BlobHash $actual) -ceq $expected) 'current source'
    $committed = git -C $Root rev-parse "$($manifest.implementation_source):$($source.path)"
    Require ($LASTEXITCODE -eq 0 -and $committed -ceq $source.git_blob) 'committed source'
}
$origin = $manifest.origin_sources[0]
$committed = git -C $Root rev-parse "$($manifest.origin_source):$($origin.path)"
Require ($LASTEXITCODE -eq 0 -and $committed -ceq $origin.git_blob) 'committed origin correction'
$fix = Get-Content -Raw (Join-Path $packet 'origin-harness-fix.patch')
Require (([regex]::Matches($fix, '(?m)^-    with_original_at\(false, None,')).Count -eq 2 -and ([regex]::Matches($fix, '(?m)^\+    with_original_at\(true, None,')).Count -eq 2) 'two original refusal expectations corrected'
Require (([regex]::Matches($fix, '(?m)^[+-](?![+-])')).Count -eq 4) 'no other source correction'
$listed = @($manifest.files.path | Sort-Object)
$actual = @(Get-ChildItem -LiteralPath $packet -File | Where-Object { $_.Name -ne 'manifest.json' -and $_.Name -notlike 'verify-result*.log' } | ForEach-Object Name | Sort-Object)
Require (($listed -join ',') -ceq ($actual -join ',')) 'exact inventory'
foreach ($item in $manifest.files) {
    Require ($item.path -cmatch '^[a-zA-Z0-9._-]+$') 'inventory basename'
    $path = Join-Path $packet $item.path
    Require ((Get-Item -LiteralPath $path).Length -eq $item.bytes) "size $($item.path)"
    Require ((Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash.ToLowerInvariant() -ceq $item.sha256) "digest $($item.path)"
}
$phases = Get-Content -Raw (Join-Path $packet 'phases.json') | ConvertFrom-Json
Require ($phases.rows.Count -eq 14) 'seven healthy controls and two spans each'
foreach ($cohort in @(@('screen','194',1), @('healthy','194',3), @('healthy','188',3))) {
    $mode=$cohort[0]; $lane=$cohort[1]; $samples=$cohort[2]
    $rows = @(Healthy (Get-Content -Raw (Join-Path $packet "$mode-$lane.log")) $samples)
    foreach ($row in $rows) {
        $derived = @($phases.rows | Where-Object { $_.mode -ceq $mode -and $_.lane -ceq $lane -and $_.sample -eq $row.sample -and $_.phase -ceq $row.phase })
        Require ($derived.Count -eq 1) 'unique derived row'
        foreach ($name in 'wall_ns','authentication_calls','authentication_ns','signature_calls','signature_ns') {
            Require ($derived[0].$name -eq $row.$name) 'derived raw value'
        }
    }
}
foreach ($binary in $manifest.binaries) {
    $lane=$binary.lane
    Refusals (Get-Content -Raw (Join-Path $packet "refusals-$lane.log"))
    $checks = Get-Content -Raw (Join-Path $packet "checks-$lane.log")
    CompilerChecks $checks $lane
    Require ((Get-Content -Raw (Join-Path $packet "final-build-$lane.log")).Contains('4 passed; 0 failed;')) 'final binary tests'
    $modes = if ($lane -ceq '194') { @('screen','healthy','refusals') } else { @('healthy','refusals') }
    foreach ($mode in $modes) {
        $raw=Get-Content -Raw (Join-Path $packet "host-$mode-$lane.log")
        Require ($raw.Contains("lane=$lane mode=$mode bytes=$($binary.bytes) sha256=$($binary.sha256)")) 'binary provenance'
        Require ($raw.Contains("HC074-FIXED-GROUPED-EXIT lane=$lane mode=$mode exit=0") -and $raw.Contains("HC074-FIXED-GROUPED-RUN lane=$lane mode=$mode completed=true guardian=false send=false")) 'host completion'
        Require (([regex]::Matches($raw,'preflight=ok supervisor_pid=8839 restarts=0')).Count -eq 2) 'host pre/post guards'
    }
}
foreach ($binary in $manifest.origin_binaries) {
    $lane=$binary.lane
    Origin (Get-Content -Raw (Join-Path $packet "origin-$lane.log"))
    $raw=Get-Content -Raw (Join-Path $packet "host-origin-$lane.log")
    Require ($raw.Contains("lane=$lane mode=origin bytes=$($binary.bytes) sha256=$($binary.sha256)")) 'corrected binary provenance'
    Require ($raw.Contains("HC074-FIXED-GROUPED-EXIT lane=$lane mode=origin exit=0") -and $raw.Contains("HC074-FIXED-GROUPED-RUN lane=$lane mode=origin completed=true guardian=false send=false")) 'corrected origin completion'
    Require (([regex]::Matches($raw,'preflight=ok supervisor_pid=8839 restarts=0')).Count -eq 2) 'corrected origin guards'
}
$negative=Get-Content -Raw (Join-Path $packet 'host-origin-initial-194.log')
Require ($negative.Contains('mode=origin exit=101') -and $negative.Contains('left: true') -and $negative.Contains('right: false') -and -not $negative.Contains('mode=origin completed=true')) 'initial wrong expectation retained as a negative'
Require ((Get-Content -Raw (Join-Path $packet 'static-contracts.log')).Contains('198 passed; 0 failed;')) 'static contracts'
Require ((Get-Content -Raw (Join-Path $packet 'registry.log')).Contains('184 discovered gates, all registered')) 'registry'
Require ((Get-Content -Raw (Join-Path $packet 'doc-check.log')).Contains('doc-check: OK')) 'documentation'
Require ((Get-Content -Raw (Join-Path $packet 'performance-contract.log')).Contains('OK (local, non-promotable)')) 'local performance contract'
Require ((Get-Content -Raw (Join-Path $packet 'prior-proof.log')).Contains('active index unchanged')) 'prior immutable evidence'
Require ((Get-Content -Raw (Join-Path $packet 'verifier-tests.log')).Contains('accepted=4 refused=13 passed=true')) 'verifier tests'
Require ((Get-Item (Join-Path $packet 'fmt.log')).Length -eq 0) 'formatting'
foreach ($name in 'windows-check.log','windows-clippy.log') {
    $raw=Get-Content -Raw (Join-Path $packet $name)
    Require ($raw.Contains('Finished') -and $raw -notmatch '(?m)^error(?:\[|:)') 'Windows package checks'
}
$post=Get-Content -Raw (Join-Path $packet 'postflight.log')
Require ($post.Contains('preflight=ok supervisor_pid=8839 restarts=0') -and $post.Contains('no_fixture_executable=true diagnostic_install_absent=true')) 'independent postflight'
$pattern='(?m)^([0-9a-f]{64})  \./bundle/([^\r\n]+)\r?$'
$before=@([regex]::Matches((Get-Content -Raw (Join-Path $packet 'setup.log')),$pattern) | ForEach-Object { "$($_.Groups[1].Value) $($_.Groups[2].Value)" } | Sort-Object -CaseSensitive)
$after=@([regex]::Matches((Get-Content -Raw (Join-Path $packet 'bundle-postflight.log')),$pattern) | ForEach-Object { "$($_.Groups[1].Value) $($_.Groups[2].Value)" } | Sort-Object -CaseSensitive)
Require ($before.Count -eq 9 -and $after.Count -eq 9 -and ($before -join ',') -ceq ($after -join ',')) 'bundle unchanged'
$scriptHashes=Get-Content -Raw (Join-Path $packet 'script-hashes.log')
foreach ($script in 'guard.sh','setup.sh','run.sh','origin.sh','postflight.sh') {
    $digest=(Get-FileHash -LiteralPath (Join-Path $packet $script) -Algorithm SHA256).Hash.ToLowerInvariant()
    Require ($scriptHashes.Contains("$digest  $($manifest.server_root)/$script")) 'executed script'
}
$qualification=(Get-FileHash -Algorithm SHA256 (Join-Path $Root 'docs/testing/performance/0.74/qualification-manifest.toml')).Hash.ToLowerInvariant()
Require ($qualification -ceq '11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc' -and $manifest.qualification_manifest_sha256 -ceq $qualification) 'qualification unchanged'
Write-Output 'HC074-FIXED-GROUPED-EVIDENCE verified=true healthy=7 persistent_refusals=28 origin_refusals=4 guardian=false send=false admission=false'
