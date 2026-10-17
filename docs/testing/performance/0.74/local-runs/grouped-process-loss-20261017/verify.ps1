param([string]$Root = (Resolve-Path "$PSScriptRoot/../../../../../..").Path,
      [switch]$CapsulesOnly,
      [switch]$SelfTest)
$ErrorActionPreference = 'Stop'
$packet = $PSScriptRoot
$baseline = '674ea58677527933b3ffd33d8f3cdda46f6bd037'
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
function CheckCapsule([string]$name, [bool]$current, [int]$expected) {
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
        if ($current) {
            $actual = [IO.File]::ReadAllText((Join-Path $Root $path)).Replace("`r", '')
            Require ($actual -ceq $rebuilt) "current source differs $path"
        }
    }
    Write-Output "HC074-CAPSULE name=$name files=$expected exact=true current=$current"
}
CheckCapsule 'instrumentation.patch' $true 5
if ($CapsulesOnly) { return }
function Cohort([string]$mode, [string]$lane, [string]$RawOverride, $DerivedOverride) {
    $raw = if ($PSBoundParameters.ContainsKey('RawOverride')) { $RawOverride } else { Get-Content -Raw (Join-Path $packet "$mode-$lane.log") }
    Require ($raw.Contains('1 passed; 0 failed;')) 'cohort test completion'
    $cases = @([regex]::Matches($raw, 'HC074-GROUPED-PROCESS seed=0x07406c17 repeat=(\d+) stage=([a-z-]+) mode=(kill|control|ack-lost) sigkill=(true|false) acks=([01]) held=true unchanged=true send=false'))
    $inspections = @([regex]::Matches($raw, 'HC074-GROUPED-INSPECTION repeat=(\d+) stage=([a-z-]+) mode=(kill|control|ack-lost) inspection=([12]) refused=true unchanged=true'))
    $expected = [Collections.Generic.List[object]]::new()
    if ($mode -ceq 'screen') {
        foreach ($kind in 'control', 'kill') {
            $expected.Add(@(0, 'returned-unacknowledged', $kind))
        }
        Require ($raw.Contains('HC074-GROUPED-PROCESS-SCREEN sigkills=1 controls=1 inspections=4 completed=true')) 'screen summary'
    } else {
        foreach ($repeat in 1..3) {
            foreach ($stage in 'returned-unacknowledged', 'bound', 'message', 'opened', 'armed', 'loaded') {
                $expected.Add(@($repeat, $stage, 'kill'))
            }
            $expected.Add(@($repeat, 'returned-unacknowledged', 'control'))
            $expected.Add(@($repeat, 'returned-unacknowledged', 'ack-lost'))
            Require ($raw.Contains("HC074-GROUPED-PROCESS-SUMMARY seed=0x07406c17 repeat=$repeat sigkills=6 controls=1 lost_acks=1 inspections=16 admission=false")) 'repeat summary'
        }
        Require (([regex]::Matches($raw, 'HC074-GROUPED-PROCESS-SUMMARY seed=')).Count -eq 3) 'three repeat summaries'
    }
    Require ($cases.Count -eq $expected.Count -and ([regex]::Matches($raw, 'HC074-GROUPED-PROCESS seed=')).Count -eq $expected.Count) 'case membership'
    Require ($inspections.Count -eq 2 * $expected.Count -and ([regex]::Matches($raw, 'HC074-GROUPED-INSPECTION repeat=')).Count -eq 2 * $expected.Count) 'inspection membership'
    $derived = if ($PSBoundParameters.ContainsKey('DerivedOverride')) { $DerivedOverride } else { Get-Content -Raw (Join-Path $packet 'cases.json') | ConvertFrom-Json }
    foreach ($i in 0..($expected.Count - 1)) {
        $e = $expected[$i]; $g = $cases[$i].Groups
        Require ([int]$g[1].Value -eq $e[0] -and $g[2].Value -ceq $e[1] -and $g[3].Value -ceq $e[2]) 'seeded exact case order'
        $kill = $e[2] -ceq 'kill'; $acks = $(if ($e[2] -ceq 'control') { 1 } else { 0 })
        Require ($g[4].Value -ceq $kill.ToString().ToLowerInvariant() -and [int]$g[5].Value -eq $acks) 'kernel/ACK outcome'
        $rows = @($derived.rows | Where-Object { $_.cohort -ceq $mode -and $_.lane -ceq $lane -and $_.repeat -eq $e[0] -and $_.stage -ceq $e[1] -and $_.mode -ceq $e[2] })
        Require ($rows.Count -eq 1 -and $rows[0].sigkill -eq $kill -and $rows[0].acks -eq $acks -and $rows[0].inspections -eq 2) 'derived exact case'
        foreach ($j in 0..1) {
            $g = $inspections[2 * $i + $j].Groups
            Require ([int]$g[1].Value -eq $e[0] -and $g[2].Value -ceq $e[1] -and $g[3].Value -ceq $e[2] -and [int]$g[4].Value -eq $j + 1) 'fresh inspection order'
        }
    }
    Write-Output "HC074-GROUPED-PROCESS-COHORT mode=$mode lane=$lane cases=$($cases.Count) inspections=$($inspections.Count) exact=true"
}
if ($SelfTest) {
    # In-memory codec fixtures, never actual process evidence or admission.
    $lines = [Collections.Generic.List[string]]::new()
    $rows = @()
    foreach ($kind in 'control', 'kill') {
        foreach ($inspection in 1..2) { $lines.Add("HC074-GROUPED-INSPECTION repeat=0 stage=returned-unacknowledged mode=$kind inspection=$inspection refused=true unchanged=true") }
        $kill = $kind -ceq 'kill'; $acks = $(if ($kill) {0} else {1})
        $lines.Add("HC074-GROUPED-PROCESS seed=0x07406c17 repeat=0 stage=returned-unacknowledged mode=$kind sigkill=$($kill.ToString().ToLowerInvariant()) acks=$acks held=true unchanged=true send=false")
        $rows += [pscustomobject]@{cohort='screen';lane='194';repeat=0;stage='returned-unacknowledged';mode=$kind;sigkill=$kill;acks=$acks;inspections=2}
    }
    $lines.Add('HC074-GROUPED-PROCESS-SCREEN sigkills=1 controls=1 inspections=4 completed=true')
    $lines.Add('test result: ok. 1 passed; 0 failed;')
    $raw = ($lines -join "`n") + "`n"
    $derived = [pscustomobject]@{rows=$rows}
    $null = Cohort 'screen' '194' -RawOverride $raw -DerivedOverride $derived
    $mutants = @(
        $raw.Replace('acks=1', 'acks=0'),
        $raw.Replace('sigkill=true', 'sigkill=false'),
        $raw.Replace('seed=0x07406c17', 'seed=0x07406c18'),
        $raw.Replace("$($lines[1])`n", ''),
        ($raw + $lines[2] + "`n"),
        $raw.Replace('stage=returned-unacknowledged', 'stage=armed'),
        $raw.Replace('repeat=0', 'repeat=1'),
        $raw.Replace('inspections=4 completed=true', 'inspections=5 completed=true')
    )
    foreach ($mutant in $mutants) {
        $refused = $false
        try { $null = Cohort 'screen' '194' -RawOverride $mutant -DerivedOverride $derived } catch { $refused = $true }
        Require $refused 'mutated process evidence must refuse'
    }
    Write-Output 'HC074-GROUPED-PROCESS-VERIFIER-TEST accepted=1 refused=8 passed=true'
    return
}
$manifest = Get-Content -Raw (Join-Path $packet 'manifest.json') | ConvertFrom-Json
Require ($manifest.baseline -ceq $baseline -and $manifest.implementation_source -ceq 'a995170b129e27db9714045012735d40610c7228') 'source cohort'
Require ($manifest.test_only -eq $true -and $manifest.seed -ceq '0x07406c17') 'test scope/seed'
Require (($manifest.points -join ',') -ceq 'returned-unacknowledged,bound,message,opened,armed,loaded') 'declared seeded order'
Require ($manifest.server_root -ceq '/tmp/hc074-grouped-process-loss-20261017.LNpZoJ') 'isolated server root'
foreach ($expectedCount in @(@('matrix_repeats_per_compiler',3), @('matrix_sigkills',36), @('matrix_controls',6), @('matrix_lost_acks',6), @('screen_sigkills',1), @('screen_controls',1), @('total_cases',50), @('total_fresh_inspections',100))) {
    Require ($manifest.($expectedCount[0]) -eq $expectedCount[1]) 'declared outcome counts'
}
foreach ($flag in 'production_changed', 'candidate_changed', 'deadlines_changed', 'guardian', 'send', 'start', 'admission', 'ship', 'systemd_ack_proven') {
    Require ($manifest.$flag -ceq $false) "scope $flag"
}
Require ($manifest.sources.Count -eq 5 -and (@($manifest.binaries.lane | Sort-Object) -join ',') -ceq '188,194') 'source/binary membership'
foreach ($source in $manifest.sources) {
    $blob = git -C $Root rev-parse "$($manifest.implementation_source):$($source.path)"
    Require ($LASTEXITCODE -eq 0 -and $blob -ceq $source.git_blob) 'committed source'
}
$listed = @($manifest.files.path | Sort-Object)
$actual = @(Get-ChildItem -LiteralPath $packet -File | Where-Object { $_.Name -ne 'manifest.json' -and $_.Name -notlike 'verify-result*.log' } | ForEach-Object Name | Sort-Object)
Require (($listed -join "`n") -ceq ($actual -join "`n")) 'exact inventory'
foreach ($item in $manifest.files) {
    Require ($item.path -cmatch '^[a-zA-Z0-9._-]+$') 'inventory basename'
    $path = Join-Path $packet $item.path
    Require ((Get-Item -LiteralPath $path).Length -eq $item.bytes) "size $($item.path)"
    Require ((Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash.ToLowerInvariant() -ceq $item.sha256) "digest $($item.path)"
}
Require ((Get-Content -Raw (Join-Path $packet 'cases.json') | ConvertFrom-Json).rows.Count -eq 50) 'all derived cases'
Cohort 'screen' '194'
Cohort 'matrix' '194'
Cohort 'matrix' '188'
foreach ($binary in $manifest.binaries) {
    $lane = $binary.lane
    $checks = Get-Content -Raw (Join-Path $packet "checks-$lane.log")
    Require ($checks.Contains("HC074-GROUPED-PROCESS-CHECKS lane=$lane completed=true") -and $checks.Contains('3 tests, 0 benchmarks')) 'focused compiler checks'
    foreach ($mode in $(if ($lane -ceq '194') { @('screen', 'matrix') } else { @('matrix') })) {
        $hostRaw = Get-Content -Raw (Join-Path $packet "host-$mode-$lane.log")
        Require ($hostRaw.Contains("lane=$lane mode=$mode bytes=$($binary.bytes) sha256=$($binary.sha256)")) 'server binary provenance'
        Require ($hostRaw.Contains("HC074-GROUPED-PROCESS-EXIT lane=$lane mode=$mode exit=0") -and $hostRaw.Contains("HC074-GROUPED-PROCESS-RUN lane=$lane mode=$mode completed=true admission=false")) 'host completion'
        Require (([regex]::Matches($hostRaw, 'preflight=ok supervisor_pid=8839 restarts=0')).Count -eq 2) 'host pre/post guards'
    }
}
Require ((Get-Content -Raw (Join-Path $packet 'contracts.log')).Contains('195 passed; 0 failed;')) 'static contracts'
Require ((Get-Content -Raw (Join-Path $packet 'registry.log')).Contains('183 discovered gates, all registered')) 'registry'
Require ((Get-Content -Raw (Join-Path $packet 'prior-proof.log')).Contains('active index unchanged')) 'immutable prior proof'
Require ((Get-Content -Raw (Join-Path $packet 'verifier-tests.log')).Contains('accepted=1 refused=8 passed=true')) 'verifier negative controls'
Require ((Get-Content -Raw (Join-Path $packet 'doc-check.log')).Contains('doc-check: OK')) 'documentation'
Require ((Get-Content -Raw (Join-Path $packet 'performance-contract.log')).Contains('OK (local, non-promotable)')) 'local performance contract'
Require ((Get-Item (Join-Path $packet 'fmt.log')).Length -eq 0) 'formatting'
$post = Get-Content -Raw (Join-Path $packet 'postflight.log')
Require ($post.Contains('preflight=ok supervisor_pid=8839 restarts=0') -and $post.Contains('no_fixture_executable=true diagnostic_install_absent=true')) 'independent executable postflight'
$bundlePattern = '(?m)^([0-9a-f]{64})  \./bundle/([^\r\n]+)\r?$'
$bundleBefore = @([regex]::Matches((Get-Content -Raw (Join-Path $packet 'setup.log')), $bundlePattern) | ForEach-Object { "$($_.Groups[1].Value) $($_.Groups[2].Value)" })
$bundleAfter = @([regex]::Matches((Get-Content -Raw (Join-Path $packet 'bundle-postflight.log')), $bundlePattern) | ForEach-Object { "$($_.Groups[1].Value) $($_.Groups[2].Value)" })
Require ($bundleBefore.Count -eq 9 -and $bundleAfter.Count -eq 9 -and ((@($bundleBefore | Sort-Object -CaseSensitive) -join "`n") -ceq (@($bundleAfter | Sort-Object -CaseSensitive) -join "`n"))) 'signed fixture data unchanged'
$scriptHashes = Get-Content -Raw (Join-Path $packet 'script-hashes.log')
foreach ($script in 'guard.sh', 'setup.sh', 'screen.sh', 'run.sh', 'postflight.sh') {
    $digest = (Get-FileHash -LiteralPath (Join-Path $packet $script) -Algorithm SHA256).Hash.ToLowerInvariant()
    Require ($scriptHashes.Contains("$digest  $($manifest.server_root)/$script")) 'executed script bytes'
}
$qualification = (Get-FileHash -Algorithm SHA256 (Join-Path $Root 'docs/testing/performance/0.74/qualification-manifest.toml')).Hash.ToLowerInvariant()
Require ($qualification -ceq '11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc' -and $manifest.qualification_manifest_sha256 -ceq $qualification) 'qualification unchanged'
Write-Output 'HC074-GROUPED-PROCESS-EVIDENCE verified=true matrix_sigkills=36 matrix_lost_acks=6 matrix_controls=6 fresh_inspections=100 production_changed=false admission=false'
