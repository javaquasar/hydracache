param([switch]$SelfTest)
$ErrorActionPreference = 'Stop'
$test = 'diagnostic_output_preparation::fixed_intent_storage::reserved_state::original_fence::tests::custody::original_fence_custody_matrix'
$cases = @('close','ack-omitted','ack-disconnected','holder-kill','original-last','repeat-refused')
function Check-Matrix([string]$raw) {
    $index = 0
    $summaries = 0
    $prefix = [regex]::Escape("test $test ... ")
    foreach ($line in ($raw -split "\r?\n")) {
        if ($line.Contains('HC074-FENCE-CUSTODY seed=')) {
            if ($line -cnotmatch "^(?:$prefix)?HC074-FENCE-CUSTODY seed=0x0740c517 repeat=([1-3]) case=([a-z-]+) original=true terminal=true retained_state=true`$") { throw 'Malformed case' }
            if ($index -ge 18 -or $summaries -ne 0 -or
                [int]$Matches[1] -ne (1 + [math]::Floor($index / 6)) -or
                $Matches[2] -cne $cases[$index % 6]) { throw 'Missing duplicate or reordered case' }
            $index++
        } elseif ($line.Contains('HC074-FENCE-CUSTODY completed=')) {
            if ($line -cne 'HC074-FENCE-CUSTODY completed=true cases=18 product=false' -or $index -ne 18) { throw 'Invalid summary' }
            $summaries++
        }
    }
    if ($index -ne 18 -or $summaries -ne 1 -or
        -not $raw.Contains('test result: ok. 1 passed; 0 failed;') -or
        $raw -cmatch 'FAILED|panicked|error:|timed out|unexpected_executable=') { throw 'Matrix incomplete or failed' }
}
function Check-Host([string]$raw) {
    if ([regex]::Matches($raw, '(?m)^HC074-FENCE-CUSTODY-COHORT.*$').Count -ne 4) { throw 'Invalid cohort envelope' }
    $last = -1
    foreach ($compiler in @('194','188')) {
        $pattern = "(?ms)^HC074-FENCE-CUSTODY-COHORT compiler=$compiler\r?\n(.*?)^HC074-FENCE-CUSTODY-COHORT compiler=$compiler completed=true\r?\n"
        $matches = [regex]::Matches($raw, $pattern)
        if ($matches.Count -ne 1 -or $matches[0].Index -le $last) { throw 'Cohort missing duplicated or reordered' }
        $last = $matches[0].Index
        Check-Matrix $matches[0].Groups[1].Value
    }
    if ([regex]::Matches($raw,'preflight=ok supervisor_pid=8839 restarts=0').Count -ne 5 -or
        [regex]::Matches($raw,'test result: ok\. 14 passed; 0 failed;').Count -ne 2 -or
        [regex]::Matches($raw,'test result: ok\. 1 passed; 0 failed;').Count -ne 2 -or
        -not $raw.TrimEnd().EndsWith('HC074-FENCE-CUSTODY-HOST completed=true supervisor_unchanged=true no_fixture_executable=true') -or
        $raw -cmatch 'FAILED|panicked|error:|timed out|unexpected_executable=') { throw 'Host envelope incomplete or failed' }
}
if ($SelfTest) {
    $rows = @()
    foreach ($repeat in 1..3) {
        foreach ($case in $cases) {
            $rows += "HC074-FENCE-CUSTODY seed=0x0740c517 repeat=$repeat case=$case original=true terminal=true retained_state=true"
        }
    }
    $summary = 'HC074-FENCE-CUSTODY completed=true cases=18 product=false'
    $good = ($rows + @($summary,'test result: ok. 1 passed; 0 failed;')) -join "`n"
    Check-Matrix $good
    $mutations = @(
        ($good -replace 'seed=0x0740c517','seed=0x0740c518'),
        ($good.Replace($rows[0] + "`n", '')),
        ($good + "`n" + $rows[0]),
        ($good.Replace($rows[0], $rows[1])),
        ($good.Replace('terminal=true','terminal=false')),
        ($good.Replace($summary, '')),
        ($good + "`nFAILED"),
        ($good + "`npanicked at fixture"),
        ($good.Replace('0 failed;', '1 failed;')),
        ($good.Replace($summary, "$summary`n$summary"))
    )
    foreach ($mutation in $mutations) {
        $refused = $false
        try { Check-Matrix $mutation } catch { $refused = $true }
        if (-not $refused) { throw 'Negative self-test accepted' }
    }
    Write-Output 'HC074-FENCE-CUSTODY-VERIFIER selftest=true positive=1 negative=10'
    return
}
$manifest = Get-Content -Raw (Join-Path $PSScriptRoot 'manifest.json') | ConvertFrom-Json
$names = @(Get-ChildItem -LiteralPath $PSScriptRoot -File | ForEach-Object { $_.Name } | Sort-Object)
$declared = @(@($manifest.files | ForEach-Object { $_.path }) + @('manifest.json') | Sort-Object)
if (($names -join '|') -cne ($declared -join '|')) { throw 'Evidence inventory differs' }
foreach ($file in $manifest.files) {
    $path = Join-Path $PSScriptRoot $file.path
    if ((Get-Item -LiteralPath $path).Length -ne $file.bytes -or
        (Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash.ToLowerInvariant() -cne $file.sha256) { throw "Evidence drift: $($file.path)" }
}
$raw = Get-Content -Raw (Join-Path $PSScriptRoot 'host.log')
Check-Host $raw
Check-Matrix (Get-Content -Raw (Join-Path $PSScriptRoot 'wsl-debug.log'))
foreach ($binary in $manifest.binaries) {
    if (-not $raw.Contains("$($binary.sha256)  tests-$($binary.compiler)")) { throw 'Host binary digest differs' }
}
foreach ($script in @('guard.sh','run.sh')) {
    $file = $manifest.files | Where-Object { $_.path -ceq $script }
    if (-not $raw.Contains("$($file.sha256)  $script")) { throw 'Executed script differs' }
}
$negative = Get-Content -Raw (Join-Path $PSScriptRoot 'baseline-negative.log')
if (-not $negative.Contains('custody_survives_original_drop') -or
    -not $negative.Contains('test result: FAILED. 0 passed; 1 failed;')) { throw 'Baseline counterexample missing' }
$repo = (& git -C $PSScriptRoot rev-parse --show-toplevel).Trim()
if ($LASTEXITCODE -ne 0) { throw 'Repository unavailable' }
foreach ($source in $manifest.sources) {
    $blob = (& git -C $repo rev-parse "$($manifest.source_commit):$($source.path)").Trim()
    if ($LASTEXITCODE -ne 0 -or $blob -cne $source.blob) { throw 'Committed source drift' }
    $current = (& git -C $repo hash-object "--path=$($source.path)" (Join-Path $repo $source.path)).Trim()
    if ($LASTEXITCODE -ne 0 -or $current -cne $blob) { throw 'Working source drift' }
}
$info = [Diagnostics.ProcessStartInfo]::new('git')
$info.UseShellExecute = $false
$info.CreateNoWindow = $true
$info.RedirectStandardOutput = $true
$info.RedirectStandardError = $true
foreach ($arg in @('-C',$repo,'diff','--binary',"$($manifest.baseline_commit)..$($manifest.source_commit)")) { $info.ArgumentList.Add($arg) }
$process = [Diagnostics.Process]::Start($info)
$expected = $process.StandardOutput.ReadToEnd()
$errors = $process.StandardError.ReadToEnd()
$process.WaitForExit()
if ($process.ExitCode -ne 0 -or $errors -ne '' -or
    $expected -cne [IO.File]::ReadAllText((Join-Path $PSScriptRoot 'implementation.patch'))) { throw 'Committed source capsule differs' }
& git -C $repo apply --reverse --check (Join-Path $PSScriptRoot 'implementation.patch')
if ($LASTEXITCODE -ne 0) { throw 'Source capsule cannot reverse current implementation' }
$qualification = Join-Path $repo 'docs/testing/performance/0.74/qualification-manifest.toml'
if ((Get-FileHash -LiteralPath $qualification -Algorithm SHA256).Hash.ToLowerInvariant() -cne $manifest.qualification_sha256) { throw 'Qualification changed' }
Write-Output 'HC074-FENCE-CUSTODY-EVIDENCE verified=true host_cases=36 local_cases=18 production_join=false admission=false'
