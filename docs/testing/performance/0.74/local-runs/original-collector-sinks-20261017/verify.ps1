$ErrorActionPreference = 'Stop'
$OutputEncoding = New-Object System.Text.UTF8Encoding($false)
[Console]::OutputEncoding = $OutputEncoding
$lf = [string][char]10
$cr = [string][char]13
$packet = $PSScriptRoot
$repo = (& git -C $packet rev-parse --show-toplevel).Trim()
if ($LASTEXITCODE -ne 0) { throw 'Repository unavailable' }
$m = Get-Content -LiteralPath (Join-Path $packet 'manifest.json') -Raw | ConvertFrom-Json
function Require($value, [string]$reason) { if (-not $value) { throw $reason } }
function Blob([string]$raw) {
    $bytes = [Text.Encoding]::UTF8.GetBytes($raw)
    $header = [Text.Encoding]::UTF8.GetBytes("blob $($bytes.Length)" + [char]0)
    $sha = [Security.Cryptography.SHA1]::Create()
    try { return ([BitConverter]::ToString($sha.ComputeHash($header + $bytes))).Replace('-', '').ToLowerInvariant() }
    finally { $sha.Dispose() }
}
function CheckScope($doc) {
    foreach ($key in @('candidate_withdrawn','runtime_restored','accepted_original_pipes_retained','deadlines_unchanged','admission_closed')) {
        Require (($doc.scope.$key -is [bool]) -and $doc.scope.$key) "Missing positive boundary $key"
    }
    foreach ($key in @('sink_constructor_active','complete_joined_pass','sink_sigkill_proof','lost_ack_proof','authenticated_start','all_writer_cessation','production_enrollment','installed_release_changed','supervisor_stopped','clock_changed','product_workload','qualification_changed','performance_claim','server_ready')) {
        Require (($doc.scope.$key -is [bool]) -and -not $doc.scope.$key) "Invalid open boundary $key"
    }
    Require ($doc.observed.first_joined_interruption_cases -eq 1) 'Wrong partial screen count'
    Require ($doc.observed.complete_joined_passes -eq 0 -and $doc.observed.sink_sigkills -eq 0 -and $doc.observed.sink_lost_ack_proofs -eq 0) 'Invented completed proof'
    Require ($doc.observed.broad_current_failures -eq 3 -and $doc.observed.broad_msrv_failures -eq 10) 'Broad reds omitted'
}
CheckScope $m
$bad = ($m | ConvertTo-Json -Depth 30 | ConvertFrom-Json)
$bad.scope.server_ready = $true
$rejected = $false
try { CheckScope $bad } catch { $rejected = $true }
Require $rejected 'Scope canary survived'
Require ($m.baseline_commit -eq '42087991f93fb1e2640e46533968ce844ddc4699') 'Wrong baseline'
$seen = @{}
foreach ($a in $m.artifacts) {
    Require (-not $seen.ContainsKey($a.path)) 'Duplicate artifact'
    Require ($a.path -notmatch '(^/|^[A-Za-z]:|\\|\.\.)') 'Unsafe artifact path'
    $seen[$a.path] = $true
    $path = Join-Path $packet $a.path
    Require (Test-Path -LiteralPath $path -PathType Leaf) "Missing $($a.path)"
    Require ((Get-Item -LiteralPath $path).Length -eq $a.bytes) "Length changed $($a.path)"
    Require ((Get-FileHash -Algorithm SHA256 -LiteralPath $path).Hash.ToLowerInvariant() -eq $a.sha256) "Hash changed $($a.path)"
}
$actual = @(Get-ChildItem -LiteralPath $packet -File -Recurse | ForEach-Object {
    $_.FullName.Substring($packet.Length + 1).Replace('\','/')
} | Where-Object { $_ -notin @('README.md','manifest.json','verify.ps1') } | Sort-Object)
Require (($actual -join '|') -eq (($m.artifacts.path | Sort-Object) -join '|')) 'Artifact inventory changed'
$records = @{}
foreach ($c in $m.commands) {
    $r = Get-Content -LiteralPath (Join-Path $packet ("logs/" + $c.key + '.json')) -Raw | ConvertFrom-Json
    Require (-not $records.ContainsKey($c.key)) 'Duplicate command'
    Require ($r.key -eq $c.key -and $r.exit_code -eq $c.exit_code -and $null -ne $r.command) "Command changed $($c.key)"
    $records[$c.key] = $r
}
$red = [IO.File]::ReadAllText((Join-Path $packet 'joined-194-first-red.log'))
Require ($red.Contains('cut=BeforeCreate') -and $red.Contains('FAILED') -and $red.Contains('0 passed; 1 failed;')) 'Stopped screen changed'
Require (-not $red.Contains('cut=RootCreated')) 'Invented RootCreated interruption'
$diag = [IO.File]::ReadAllText((Join-Path $packet 'attribution-guarded.log'))
Require ($diag.Contains('visited=[BeforeCreate] error=Original(Original(Original(Causal(Time)))) accepted=false')) 'Original causal refusal missing'
Require ($diag.Contains('root=false io=NotFound') -and $diag.Contains('1 passed; 0 failed;')) 'Diagnostic scope changed'
Require ($records['sinks-server-run'].exit_code -ne 0 -and $records['sinks-server-run'].output.Length -eq 0) 'Setup red omitted'
Require ($records['sinks-dispatch-audit'].output.Contains('705:1000:1000 .')) 'Unsafe temporary mode not retained'
Require ($records['sinks-temp-transport-audit'].output.Contains('\r')) 'CR-suffixed directories not retained'
Require ($records['sinks-postflight'].output.Contains('postflight=ok original_test_processes=absent')) 'Postflight incomplete'
foreach ($lane in @('194','188')) {
    Require ($records["sinks-linux-final$lane"].exit_code -ne 0) "Broad $lane red omitted"
    Require ($records["sinks-focused$lane"].exit_code -eq 0) "Narrow $lane failed"
    Require ($records["sinks-rollback-linux$lane"].exit_code -eq 0 -and $records["sinks-rollback-win$lane"].exit_code -eq 0) "Rollback $lane failed"
    Require ($records["sinks-focused$lane"].output.Contains('64 passed; 0 failed;')) 'Candidate compile-fail coverage changed'
    Require ($records["sinks-rollback-linux$lane"].output.Contains('57 passed; 0 failed;')) 'Restored compile-fail coverage changed'
}
Require ($records['sinks-rollback-static'].output.Contains('187 passed; 0 failed;')) 'Restored static gate missing'
Require ($records['sinks-rollback-registry'].output.Contains('180 discovered gates, all registered')) 'Restored registry missing'
Require ($records['sinks-ship-guard'].exit_code -ne 0) 'Ship guard unexpectedly green'
$patch = [IO.File]::ReadAllText((Join-Path $packet 'candidate-1.patch')).Replace($cr,'')
$blocks = @([regex]::Split($patch, '(?m)(?=^diff --git )') | Where-Object { $_.StartsWith('diff --git ') })
Require ($blocks.Count -eq 14 -and $m.source_blobs.Count -eq 14) 'Wrong candidate inventory'
$rebuilt = @{}
foreach ($block in $blocks) {
    $match = [regex]::Match($block, '(?m)^\+\+\+ b/(.+)$')
    Require $match.Success 'Missing candidate path'
    $path = $match.Groups[1].Value
    Require (-not $rebuilt.ContainsKey($path)) 'Duplicate candidate source'
    $newFile = $block.Contains('--- /dev/null' + $lf)
    if ($newFile) { $old = @() } else {
        $old = @(& git -C $repo show ($m.baseline_commit + ':' + $path))
        Require ($LASTEXITCODE -eq 0) "Missing old source $path"
    }
    $output = New-Object 'System.Collections.Generic.List[string]'
    $cursor = 0
    foreach ($h in @([regex]::Split($block, '(?m)(?=^@@ )') | Where-Object { $_.StartsWith('@@ ') })) {
        $lines = $h.TrimEnd([char]10).Split([char]10)
        $header = [regex]::Match($lines[0], '^@@ -(\d+)(?:,(\d+))? \+(\d+)(?:,(\d+))? @@')
        Require $header.Success 'Bad hunk'
        $startIndex = [Math]::Max(0, [int]$header.Groups[1].Value - 1)
        while ($cursor -lt $startIndex) { $output.Add([string]$old[$cursor]); $cursor++ }
        foreach ($line in @($lines | Select-Object -Skip 1)) {
            if ($line.Length -eq 0) { continue }
            $prefix = $line.Substring(0,1)
            Require ($prefix -in @(' ','+','-')) 'Unsupported patch record'
            $body = $line.Substring(1)
            if ($prefix -ne '+') {
                Require ($cursor -lt $old.Count -and [string]$old[$cursor] -ceq $body) "Old context differs $path"
                $cursor++
            }
            if ($prefix -ne '-') { $output.Add($body) }
        }
    }
    while ($cursor -lt $old.Count) { $output.Add([string]$old[$cursor]); $cursor++ }
    $raw = [string]::Join($lf, $output.ToArray()) + $lf
    $wanted = @($m.source_blobs | Where-Object { $_.path -ceq $path })
    Require ($wanted.Count -eq 1 -and (Blob $raw) -ceq $wanted[0].git_blob) "Candidate blob differs $path"
    $rebuilt[$path] = $true
    if ($newFile) {
        Require (-not (Test-Path -LiteralPath (Join-Path $repo $path))) "Withdrawn source active $path"
    } else {
        & git -C $repo diff --exit-code $m.baseline_commit -- $path
        Require ($LASTEXITCODE -eq 0) "Withdrawn change active $path"
    }
}
$attribution = [IO.File]::ReadAllText((Join-Path $packet 'attribution-only-test.rs.txt')).Replace($cr,'')
Require ((Blob $attribution) -eq $m.attribution_test_git_blob) 'Attribution-only source changed'
Write-Output 'Original sink negative proof intact: candidate WITHDRAWN; causal expiry before output; new SIGKILL/lost-ack proof ABSENT; runtime restored, admission CLOSED'
