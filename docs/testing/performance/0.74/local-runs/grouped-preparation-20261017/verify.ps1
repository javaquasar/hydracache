param([string]$Root = (Resolve-Path "$PSScriptRoot/../../../../../..").Path,
      [switch]$CapsulesOnly)
$ErrorActionPreference = 'Stop'
$packet = $PSScriptRoot
$baseline = '67419018a60843da0825324f2cb82e67d6c12ddc'
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
CheckCapsule 'candidate.patch' $true 6
if ($CapsulesOnly) { return }
$manifest = Get-Content -Raw (Join-Path $packet 'manifest.json') | ConvertFrom-Json
Require ($manifest.baseline -ceq $baseline -and $manifest.implementation_source -ceq '76e8a1b71f50eff3eb8febfd775b7b2358049437') 'source cohort'
Require ($manifest.test_only -eq $true -and $manifest.candidate_observation_schedule_changed -eq $true) 'candidate scope'
foreach ($flag in 'production_changed', 'deadlines_changed', 'guardian', 'send', 'start', 'admission', 'ship') {
    Require ($manifest.$flag -ceq $false) "scope $flag"
}
Require ($manifest.sources.Count -eq 6 -and $manifest.binaries.Count -eq 2) 'provenance membership'
Require ((@($manifest.binaries.lane | Sort-Object) -join ',') -ceq '188,194') 'exact compiler membership'
foreach ($source in $manifest.sources) {
    $committed = git -C $Root rev-parse "$($manifest.implementation_source):$($source.path)"
    Require ($LASTEXITCODE -eq 0 -and $committed -ceq $source.git_blob) 'committed source'
}
$listed = @($manifest.files.path | Sort-Object)
$actual = @(Get-ChildItem -LiteralPath $packet -File | Where-Object { $_.Name -ne 'manifest.json' -and $_.Name -notlike 'verify-result*.log' } | ForEach-Object Name | Sort-Object)
Require (($listed -join "`n") -ceq ($actual -join "`n")) 'exact file inventory'
foreach ($item in $manifest.files) {
    Require ($item.path -cmatch '^[a-zA-Z0-9._-]+$') 'inventory basename'
    $path = Join-Path $packet $item.path
    Require ((Get-Item -LiteralPath $path).Length -eq $item.bytes) "size $($item.path)"
    Require ((Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash.ToLowerInvariant() -ceq $item.sha256) "digest $($item.path)"
}
$phases = Get-Content -Raw (Join-Path $packet 'phases.json') | ConvertFrom-Json
Require ($phases.rows.Count -eq 14) 'seven healthy controls, two spans each'
foreach ($cohort in @(@('screen', '194', 1), @('healthy', '194', 3), @('healthy', '188', 3))) {
    $mode = $cohort[0]; $lane = $cohort[1]; $samples = $cohort[2]
    $raw = Get-Content -Raw (Join-Path $packet "$mode-$lane.log")
    Require ($raw.Contains('1 passed; 0 failed;')) 'healthy completion'
    Require (([regex]::Matches($raw, 'HC074-GROUPED-CONTROL sample=')).Count -eq $samples) 'healthy control membership'
    Require (([regex]::Matches($raw, 'HC074-GROUPED-COST sample=')).Count -eq 2 * $samples) 'healthy span membership'
    foreach ($sample in 0..($samples - 1)) {
        Require ($raw.Contains("HC074-GROUPED-CONTROL sample=$sample original=true unchanged=true checkpoints=5 bounds_original=true reference=true guardian=false send=false admission=false")) 'healthy original invariants'
        foreach ($phase in 'prepare', 'reference-validate') {
            $marker = "HC074-GROUPED-COST sample=$sample phase=$phase report="
            $lines = @($raw.Split("`n") | Where-Object { $_.Contains($marker) })
            Require ($lines.Count -eq 1 -and $lines[0].Contains('first_time: None, time_refusals: 0')) 'unrefused raw report'
            $stats = @([regex]::Matches($lines[0], 'Stats \{ wall_ns: (\d+), authentication_calls: (\d+), authentication_ns: (\d+), signature_calls: (\d+), signature_ns: (\d+), worker_policy_calls: (\d+), worker_policy_ns: (\d+) \}'))
            Require ($stats.Count -eq 8) 'exact trace stage count'
            $active = $(if ($phase -ceq 'prepare') { 6 } else { 7 })
            for ($i = 0; $i -lt 8; $i++) {
                $g = $stats[$i].Groups
                $calls = $(if ($i -eq $active) { 34 } else { 0 })
                Require ([long]$g[2].Value -eq $calls -and [long]$g[4].Value -eq $calls) 'all stage IPC counters'
                Require ([long]$g[6].Value -eq 0 -and [long]$g[7].Value -eq 0) 'unused worker counters'
            }
            $rows = @($phases.rows | Where-Object { $_.mode -ceq $mode -and $_.lane -ceq $lane -and $_.sample -eq $sample -and $_.phase -ceq $phase })
            Require ($rows.Count -eq 1) 'unique derived span'
            $g = $stats[$active].Groups; $row = $rows[0]
            Require ($row.wall_ns -eq [long]$g[1].Value -and $row.authentication_ns -eq [long]$g[3].Value -and $row.signature_ns -eq [long]$g[5].Value) 'derived numeric spans'
            Require ($row.authentication_calls -eq 34 -and $row.signature_calls -eq 34) 'derived counters'
        }
    }
}
$expected = @('trust/opened', 'trust/loaded', 'trust/bound', 'trust/message', 'trust/armed', 'bundle-content/loaded', 'bundle-inode/armed', 'output/armed', 'projection/armed', 'state/armed', 'writer/armed', 'message/armed', 'alarm/armed', 'expiry/armed')
foreach ($binary in $manifest.binaries) {
    $lane = $binary.lane
    Require ($lane -cin @('194', '188')) 'compiler lane'
    $raw = Get-Content -Raw (Join-Path $packet "refusals-$lane.log")
    Require ($raw.Contains('1 passed; 0 failed;')) 'refusal family completion'
    $faultRows = @([regex]::Matches($raw, 'HC074-GROUPED-REFUSAL case=([a-z-]+) cut=([a-z]+) exact=true sticky=true held=true unchanged=true send=false'))
    Require ($faultRows.Count -eq 14 -and ([regex]::Matches($raw, 'HC074-GROUPED-REFUSAL case=')).Count -eq 14) 'refusal row membership'
    for ($i = 0; $i -lt 14; $i++) {
        Require (("$($faultRows[$i].Groups[1].Value)/$($faultRows[$i].Groups[2].Value)") -ceq $expected[$i]) 'refusal order and cut'
    }
    $checks = Get-Content -Raw (Join-Path $packet "checks-$lane.log")
    Require ($checks.Contains("HC074-GROUPED-CHECKS lane=$lane completed=true") -and $checks.Contains('3 tests, 0 benchmarks')) 'focused compiler checks and gate discovery'
    foreach ($mode in 'healthy', 'refusals') {
        $hostRaw = Get-Content -Raw (Join-Path $packet "host-$mode-$lane.log")
        Require ($hostRaw.Contains("lane=$lane mode=$mode bytes=$($binary.bytes) sha256=$($binary.sha256)")) 'server binary provenance'
        Require ($hostRaw.Contains("HC074-GROUPED-EXIT lane=$lane mode=$mode exit=0") -and $hostRaw.Contains("HC074-GROUPED-RUN lane=$lane mode=$mode completed=true guardian=false send=false")) 'host run admission'
        Require (([regex]::Matches($hostRaw, 'preflight=ok supervisor_pid=8839 restarts=0')).Count -eq 2) 'host pre/post guards'
    }
}
Require ((Get-Content -Raw (Join-Path $packet 'contracts.log')).Contains('194 passed; 0 failed;')) 'static contracts'
$screenHost = Get-Content -Raw (Join-Path $packet 'host-screen-194.log')
Require ($screenHost.Contains('HC074-GROUPED-EXIT lane=194 mode=screen exit=0') -and $screenHost.Contains('HC074-GROUPED-RUN lane=194 mode=screen completed=true guardian=false send=false')) 'first screen gate'
Require (([regex]::Matches($screenHost, 'preflight=ok supervisor_pid=8839 restarts=0')).Count -eq 2) 'screen pre/post guards'
Require ((Get-Content -Raw (Join-Path $packet 'registry.log')).Contains('183 discovered gates, all registered')) 'registry'
Require ((Get-Content -Raw (Join-Path $packet 'prior-proof.log')).Contains('active index unchanged')) 'unchanged prior proof'
Require ((Get-Content -Raw (Join-Path $packet 'doc-check.log')).Contains('doc-check: OK')) 'documentation'
Require ((Get-Content -Raw (Join-Path $packet 'performance-contract.log')).Contains('OK (local, non-promotable)')) 'local performance contract'
Require ((Get-Item (Join-Path $packet 'fmt.log')).Length -eq 0) 'formatting'
$postflight = Get-Content -Raw (Join-Path $packet 'postflight.log')
Require ($postflight.Contains('preflight=ok supervisor_pid=8839 restarts=0') -and $postflight.Contains('no_fixture_executable=true diagnostic_install_absent=true')) 'independent postflight'
$scriptHashes = Get-Content -Raw (Join-Path $packet 'script-hashes.log')
foreach ($script in 'guard.sh', 'setup.sh', 'run.sh', 'postflight.sh') {
    $digest = (Get-FileHash -LiteralPath (Join-Path $packet $script) -Algorithm SHA256).Hash.ToLowerInvariant()
    Require ($scriptHashes.Contains("$digest  $($manifest.server_root)/$script")) 'executed script bytes'
}
Require ((Get-FileHash -Algorithm SHA256 (Join-Path $Root 'docs/testing/performance/0.74/qualification-manifest.toml')).Hash.ToLowerInvariant() -ceq '11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc') 'qualification identity'
Write-Output 'HC074-GROUPED-EVIDENCE verified=true screen=1 repeated=6 refusals=28 production_changed=false admission=false'
