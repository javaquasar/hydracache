param([string]$Root = (Resolve-Path "$PSScriptRoot/../../../../../..").Path,
      [switch]$SelfTest)
$ErrorActionPreference = 'Stop'
function Require($ok, [string]$message) { if (-not $ok) { throw $message } }
function ParseSummary([string]$raw) {
    $lines = @($raw.Replace("`r", '').TrimEnd("`n").Split("`n"))
    Require ($lines.Count -ge 5 -and $lines[0] -cmatch '^% time\s+seconds\s+usecs/call\s+calls\s+errors syscall$') 'summary header'
    Require ($lines[1] -cmatch '^[- ]+$' -and $lines[-2] -ceq $lines[1]) 'summary separators'
    $rows = [Collections.Generic.List[object]]::new()
    $names = @{}
    foreach ($line in $lines[2..($lines.Count - 3)] + @($lines[-1])) {
        Require ($line -cmatch '^\s*(\d+\.\d{2})\s+(\d+\.\d{6})\s+(\d+)\s+(\d+)\s+(?:(\d+)\s+)?([a-z0-9_]+)$') 'summary row syntax'
        $name = $Matches[6]
        Require (-not $names.ContainsKey($name)) 'duplicate syscall'
        $names[$name] = $true
        $row = [pscustomobject]@{
            syscall = $name; percent = [decimal]::Parse($Matches[1], [cultureinfo]::InvariantCulture)
            seconds = [decimal]::Parse($Matches[2], [cultureinfo]::InvariantCulture)
            usecs_per_call = [long]$Matches[3]; calls = [long]$Matches[4]
            errors = $(if ($Matches[5]) { [long]$Matches[5] } else { 0L })
        }
        Require ($row.calls -gt 0 -and $row.errors -le $row.calls) 'summary call/error bounds'
        $rows.Add($row)
    }
    Require ($rows[-1].syscall -ceq 'total' -and $rows[-1].percent -eq 100) 'summary total last'
    $body = @($rows | Where-Object syscall -CNE total)
    Require ($body.Count -eq $rows.Count - 1) 'summary total membership'
    Require (($body | Measure-Object calls -Sum).Sum -eq $rows[-1].calls) 'summary call total'
    Require (($body | Measure-Object errors -Sum).Sum -eq $rows[-1].errors) 'summary error total'
    # Each printed row is rounded to six fractional decimal places.
    Require ([Math]::Abs(($body | Measure-Object seconds -Sum).Sum - $rows[-1].seconds) -le ($body.Count + 1) * 0.000001) 'summary rounded seconds total'
    return $rows.ToArray()
}
if ($SelfTest) {
    $valid = "% time     seconds  usecs/call     calls    errors syscall`n------ ----------- ----------- --------- --------- ------------------`n100.00    0.000002           1         2         1 access`n------ ----------- ----------- --------- --------- ------------------`n100.00    0.000002           1         2         1 total`n"
    Require (@(ParseSummary $valid).Count -eq 2) 'valid summary'
    $mutants = @(
        $valid.Replace('errors syscall', 'arguments syscall'),
        $valid.Replace('2         1 total', '3         1 total'),
        $valid.Replace('2         1 total', '2         0 total'),
        $valid.Replace('0.000002           1         2         1 total', '0.000102           1         2         1 total'),
        $valid.Replace('2         1 access', '2         3 access'),
        $valid.Replace(' access', ' total'),
        $valid.Replace(' access', ' /secret/path'),
        $valid.Replace('100.00    0.000002           1         2         1 access', "100.00    0.000002           1         2         1 access`n100.00    0.000002           1         2         1 access")
    )
    foreach ($mutant in $mutants) {
        $refused = $false
        try { $null = ParseSummary $mutant } catch { $refused = $true }
        Require $refused 'mutated summary must refuse'
    }
    Write-Output 'HC074-SYSCALL-VERIFIER-TEST valid=1 refused=8 passed=true'
    return
}
$packet = $PSScriptRoot
$manifest = Get-Content -Raw (Join-Path $packet 'manifest.json') | ConvertFrom-Json
$source = 'b05bb9a2065627f560958517320dfca126d1b35f'
Require ($manifest.source -ceq $source -and $manifest.preregistration -ceq 'fc3c19b1') 'source/protocol'
foreach ($flag in 'production_changed', 'schedule_changed', 'deadlines_changed', 'reservation', 'admission', 'ship') {
    Require ($manifest.$flag -ceq $false) "scope $flag"
}
Require ($manifest.server_root -ceq '/tmp/hc074-controller-syscall-20261017.b2iAl8') 'server root'
Require ($manifest.binaries.Count -eq 2 -and $manifest.source_trees.Count -eq 2) 'provenance membership'
Require ((@($manifest.binaries.lane | Sort-Object) -join ',') -ceq '188,194') 'exact compiler membership'
Require ((@($manifest.source_trees.path | Sort-Object) -join ',') -ceq 'crates/xtask,tools/long-run-supervisor-074') 'exact source membership'
Require ((Get-FileHash (Join-Path $packet '../controller-component-cost-20261017/manifest.json') -Algorithm SHA256).Hash.ToLowerInvariant() -ceq '68983964182435ceadf1750eaf9eeacc624a88aff58cdae0d1e3500e6fa4ea64') 'prior provenance digest'
foreach ($tree in $manifest.source_trees) {
    Require ($tree.path -cin @('tools/long-run-supervisor-074', 'crates/xtask')) 'source tree path'
    $committed = git -C $Root rev-parse "${source}:$($tree.path)"
    Require ($LASTEXITCODE -eq 0 -and $committed -ceq $tree.git_tree) 'committed source tree'
    $changed = @(git -C $Root diff --name-only $source -- $tree.path)
    Require ($LASTEXITCODE -eq 0 -and $changed.Count -eq 0) 'unchanged current runtime and tests'
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
$scriptHashes = Get-Content -Raw (Join-Path $packet 'script-hashes.log')
foreach ($script in 'guard.sh', 'run.sh', 'postflight.sh') {
    $digest = (Get-FileHash -LiteralPath (Join-Path $packet $script) -Algorithm SHA256).Hash.ToLowerInvariant()
    Require ($scriptHashes.Contains("$digest  $($manifest.server_root)/$script")) 'executed script bytes'
}
Require ((Get-Content -Raw (Join-Path $packet 'contracts.log')).Contains('193 passed; 0 failed;')) 'static contracts'
Require ((Get-Content -Raw (Join-Path $packet 'doc-check.log')).Contains('doc-check: OK')) 'documentation'
Require ((Get-Content -Raw (Join-Path $packet 'performance-contract.log')).Contains('OK (local, non-promotable)')) 'local performance contract'
Require ((Get-Item (Join-Path $packet 'fmt-wsl.log')).Length -eq 0) 'WSL formatting'
Require ((Get-Content -Raw (Join-Path $packet 'fmt.log')).Contains('os error 206')) 'retained Windows formatting limitation'
Require ((Get-Content -Raw (Join-Path $packet 'verifier-tests.log')).Contains('valid=1 refused=8 passed=true')) 'verifier self-test'
Require ((Get-Content -Raw (Join-Path $packet 'prior-proof.log')).Contains('verified=true controls=6 rows=60')) 'unchanged prior verifier'
$names = @('worker-live', 'controller-source', 'signed-controller', 'builder-body-data-only', 'build-validator', 'oidc-source', 'oidc-token-data-only', 'oidc-validator', 'authority-binding', 'worker-time')
$counts = @(0, 0, 1, 0, 2, 0, 0, 4, 0, 0)
$hostRaw = Get-Content -Raw (Join-Path $packet 'host.log')
Require ($hostRaw.Contains('strace -- version 6.8') -and $hostRaw.Contains('HC074-SYSCALL-ENV perf_event_paranoid=4')) 'tracer environment'
$exitOrder = @([regex]::Matches($hostRaw, 'HC074-SYSCALL-EXIT invocation=(\d+-(?:plain|traced)) exit=0') | ForEach-Object { $_.Groups[1].Value })
Require (($exitOrder -join ',') -ceq '194-plain,194-traced,188-traced,188-plain') 'invocation order'
Require (([regex]::Matches($hostRaw, 'HC074-SYSCALL-SERVER preflight=ok supervisor_pid=8839 restarts=0')).Count -eq 9) 'pre/post guards'
Require ($hostRaw.Contains('HC074-SYSCALL-FAMILY invocations=4 completed=true')) 'completed family'
Require (([regex]::Matches($hostRaw, 'HC074-SYSCALL-CONTROL invocation=')).Count -eq 4) 'four invocation controls'
foreach ($binary in $manifest.binaries) {
    Require ($binary.lane -cin @('194', '188')) 'compiler lane'
    Require ($hostRaw.Contains("$($binary.sha256)  ./tests-$($binary.lane)")) 'installed test hash'
    $prior = Get-Content -Raw (Join-Path $packet '../controller-component-cost-20261017/manifest.json') | ConvertFrom-Json
    $pinned = @($prior.binaries | Where-Object lane -CEQ $binary.lane)
    Require ($pinned.Count -eq 1 -and $binary.sha256 -ceq $pinned[0].sha256 -and $binary.bytes -eq $pinned[0].bytes) 'reused binary provenance'
    foreach ($mode in 'plain', 'traced') {
        $invocation = "$($binary.lane)-$mode"
        $raw = Get-Content -Raw (Join-Path $packet "run-$invocation.log")
        $rowPattern = 'HC074-COMPONENT-COST sample=(\d+) order=(\d+) component=([a-z0-9-]+) stats=Stats \{ wall_ns: (\d+), authentication_calls: (\d+), authentication_ns: (\d+), signature_calls: (\d+), signature_ns: (\d+), worker_policy_calls: (\d+), worker_policy_ns: (\d+) \}'
        $rows = @([regex]::Matches($raw, $rowPattern))
        Require ($rows.Count -eq 30 -and ([regex]::Matches($raw, 'HC074-COMPONENT-COST sample=')).Count -eq 30) 'thirty component rows'
        Require (([regex]::Matches($raw, 'HC074-COMPONENT-CONTROL sample=')).Count -eq 3) 'three original controls'
        for ($i = 0; $i -lt 30; $i++) {
            $sample = [int][Math]::Floor($i / 10); $order = $i % 10
            $index = ($order + $sample * 3) % 10; $g = $rows[$i].Groups
            Require ([int]$g[1].Value -eq $sample -and [int]$g[2].Value -eq $order -and $g[3].Value -ceq $names[$index]) 'component membership/order'
            Require ([long]$g[4].Value -gt 0 -and [int]$g[5].Value -eq $counts[$index] -and [int]$g[7].Value -eq $counts[$index]) 'component counters'
            Require ([long]$g[9].Value -eq 0 -and [long]$g[10].Value -eq 0) 'unused worker counters'
        }
        foreach ($sample in 0..2) {
            Require ($raw.Contains("HC074-COMPONENT-CONTROL sample=$sample components=10 original=true unchanged=true reservation=false production_changed=false schedule_changed=false admission=false")) 'unchanged original/no reservation'
        }
        Require ($raw -match 'test result: ok\. 1 passed; 0 failed; 0 ignored; 0 measured; 1043 filtered out; finished in (\d+\.\d+)s') 'healthy test completion'
        Write-Output "HC074-SYSCALL-SPAN invocation=$invocation family_seconds=$($Matches[1]) rows=30"
    }
    $syscalls = @(ParseSummary (Get-Content -Raw (Join-Path $packet "syscalls-$($binary.lane).log")))
    foreach ($key in @(@('statx', 145473), @('openat', 127037), @('fstatfs', 140808), @('close', 131444), @('pread64', 27767), @('fsync', 18), @('access', 1))) {
        $entry = @($syscalls | Where-Object syscall -CEQ $key[0])
        Require ($entry.Count -eq 1 -and $entry[0].calls -eq $key[1]) "observed syscall count $($key[0])"
    }
    Require ($syscalls[-1].errors -eq 1) 'one aggregate syscall error'
    Require (@($syscalls | Where-Object { $_.syscall -cne 'total' -and $_.errors -gt 0 }).syscall -ceq 'access') 'error membership'
    Write-Output "HC074-SYSCALL-SUMMARY lane=$($binary.lane) calls=$($syscalls[-1].calls) errors=1 rows=$($syscalls.Count - 1)"
}
$post = Get-Content -Raw (Join-Path $packet 'postflight.log')
Require ($post.Contains('preflight=ok supervisor_pid=8839 restarts=0') -and $post.Contains('no_fixture_executable=true diagnostic_install_absent=true')) 'independent postflight'
$qualification = (Get-FileHash -Algorithm SHA256 (Join-Path $Root 'docs/testing/performance/0.74/qualification-manifest.toml')).Hash.ToLowerInvariant()
Require ($qualification -ceq '11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc' -and $manifest.qualification_manifest_sha256 -ceq $qualification) 'qualification unchanged'
Write-Output 'HC074-SYSCALL-EVIDENCE verified=true controls=12 rows=120 production_changed=false admission=false'
