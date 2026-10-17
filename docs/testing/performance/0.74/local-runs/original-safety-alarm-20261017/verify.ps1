param([string]$Root = (Resolve-Path "$PSScriptRoot/../../../../../..").Path,
      [switch]$CapsulesOnly)
$ErrorActionPreference = 'Stop'
$packet = $PSScriptRoot
$baseline = '4c33c5ca97ef38e877e1a030fbb3fd06f676b8dc'
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
CheckCapsule 'candidate-c1.patch' $false 10
CheckCapsule 'attribution.patch' $false 10
CheckCapsule 'diagnostic-final.patch' $true 9
if ($CapsulesOnly) { return }
$manifest = Get-Content -Raw (Join-Path $packet 'manifest.json') | ConvertFrom-Json
Require ($manifest.accepted -eq $false -and $manifest.production_api -eq $false) 'no production admission'
Require ($manifest.baseline -ceq $baseline) 'baseline'
foreach ($source in $manifest.sources) {
    $committed = git -C $Root rev-parse "$($manifest.implementation_source):$($source.path)"
    Require ($LASTEXITCODE -eq 0 -and $committed -ceq $source.git_blob) 'committed source identity'
    $current = [IO.File]::ReadAllText((Join-Path $Root $source.path)).Replace("`r", '')
    Require ((BlobHash $current) -ceq $source.git_blob) 'active source identity'
}
foreach ($item in $manifest.files) {
    $path = Join-Path $packet $item.path
    Require ((Get-Item -LiteralPath $path).Length -eq $item.bytes) "size $($item.path)"
    Require ((Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash.ToLowerInvariant() -ceq $item.sha256) "digest $($item.path)"
}
$negative = Get-Content -Raw (Join-Path $packet 'healthy-194.log')
Require ($negative.Contains('Causal(Time)') -and $negative.Contains('0 passed; 1 failed;')) 'negative healthy screen'
$diagnostic = Get-Content -Raw (Join-Path $packet 'diagnostic-final.log')
Require ($diagnostic.Contains('1 passed; 0 failed;')) 'diagnostic completion'
foreach ($sample in 0..2) {
    Require ($diagnostic.Contains("HC074-ALARM-ATTRIBUTION-CLOSED sample=$sample accepted=false renewed=false guardian=false send=false")) 'diagnostic membership'
}
Require (([regex]::Matches($diagnostic, 'phase=revalidate.*result=Err\(.*Causal\(Time\)')).Count -eq 3) 'three expected refusals'
$postflight = Get-Content -Raw (Join-Path $packet 'postflight-executable.log')
Require ($postflight.Contains('preflight=ok supervisor_pid=8839 restarts=0') -and $postflight.Contains('no_fixture_executable=true diagnostic_install_absent=true')) 'executable postflight'
$phases = Get-Content -Raw (Join-Path $packet 'phases.json') | ConvertFrom-Json
Require ($phases.rows.Count -eq 15 -and $phases.samples -eq 3) 'phase inventory'
foreach ($sample in 0..2) {
    foreach ($phase in 'outputs', 'material', 'message', 'alarm', 'revalidate') {
        $rows = @($phases.rows | Where-Object { $_.sample -eq $sample -and $_.phase -ceq $phase })
        Require ($rows.Count -eq 1) 'unique phase row'
        $row = $rows[0]
        $line = @($diagnostic.Split("`n") | Where-Object { $_.Contains("HC074-ALARM-ATTRIBUTION sample=$sample phase=$phase ") })
        Require ($line.Count -eq 1) 'raw phase row'
        $stats = "wall_ns: $($row.wall_ns), authentication_calls: $($row.authentications), authentication_ns: $($row.authentication_ns), signature_calls: $($row.signatures), signature_ns: $($row.signature_ns)"
        Require ($line[0].Contains($stats)) 'raw phase values'
    }
}
foreach ($lane in '194', '188') {
    $log = Get-Content -Raw (Join-Path $packet "final-checks-$lane.log")
    Require ($log.Contains("HC074-ALARM-FINAL-CHECKS lane=$lane completed=true production_api=false")) "final lane $lane"
}
Require ((Get-FileHash -Algorithm SHA256 (Join-Path $Root 'docs/testing/performance/0.74/qualification-manifest.toml')).Hash.ToLowerInvariant() -ceq '11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc') 'qualification identity'
Write-Output 'HC074-SAFETY-ALARM-EVIDENCE verified=true accepted=false guardian=false ship=false'
