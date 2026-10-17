$ErrorActionPreference = 'Stop'
$OutputEncoding = New-Object System.Text.UTF8Encoding($false)
[Console]::OutputEncoding = $OutputEncoding
$packet = $PSScriptRoot
$repo = (& git -C $packet rev-parse --show-toplevel).Trim()
if ($LASTEXITCODE -ne 0) { throw 'Repository unavailable' }
$lf = [string][char]10
function Require($value, [string]$reason) { if (-not $value) { throw $reason } }
function Blob([string]$raw) {
    $bytes = [Text.Encoding]::UTF8.GetBytes($raw)
    $header = [Text.Encoding]::UTF8.GetBytes("blob $($bytes.Length)" + [char]0)
    $sha = [Security.Cryptography.SHA1]::Create()
    try { return ([BitConverter]::ToString($sha.ComputeHash($header + $bytes))).Replace('-', '').ToLowerInvariant() }
    finally { $sha.Dispose() }
}
function Scope($scope) {
    foreach ($key in @('attribution_complete','joined_complete','process_loss_complete','deadlines_unchanged','original_guards_unchanged','admission_closed')) {
        Require (($scope.$key -is [bool]) -and $scope.$key) "Missing boundary $key"
    }
    foreach ($key in @('authenticated_start','writer_handoff','all_writer_cessation','completion','later_cells','production_enrollment','installed_release_changed','supervisor_stopped','clock_changed','product_workload','qualification_changed','performance_claim','server_ready')) {
        Require (($scope.$key -is [bool]) -and -not $scope.$key) "Invalid open boundary $key"
    }
}
function Count([string]$raw, [string]$pattern, [int]$wanted) {
    Require ([regex]::Matches($raw, $pattern, [Text.RegularExpressions.RegexOptions]::Multiline).Count -eq $wanted) "Wrong record count: $pattern"
}
function Crash([string]$raw) {
    $stages = @(
        'Starting(BeforeCreate)','Starting(Created)','Starting(CreateDirectorySynced)',
        'Starting(PrefixWritten)','Starting(BytesWritten)','Starting(Sealed)',
        'Starting(FileSynced)','Starting(DirectorySynced)','Starting(Acknowledging)',
        'Pipes(BeforeCreate)','Pipes(StdoutCreated)','Pipes(StdoutReady)',
        'Pipes(StderrCreated)','Pipes(StderrReady)','Pipes(Acknowledging)',
        'Sinks(BeforeCreate)','Sinks(RootCreated)','Sinks(RootSynced)',
        'Sinks(DirectoryCreated)','Sinks(DirectorySynced)','Sinks(StdoutCreated)',
        'Sinks(StdoutSynced)','Sinks(StderrCreated)','Sinks(StderrSynced)',
        'Sinks(Acknowledging)','AfterReturn'
    )
    $rows = [regex]::Matches($raw, 'original_output_join_crash seed=0x074051d1 repeat=([123]) stage=(\S+) sigkill=(true|false) fresh_inspection=([12]) empty=false unchanged=true$', [Text.RegularExpressions.RegexOptions]::Multiline)
    Require ($rows.Count -eq 162) 'Incomplete process-loss observations'
    $seen = @{}
    foreach ($row in $rows) {
        $key = ($row.Groups[1..4].Value -join '|')
        Require (-not $seen.ContainsKey($key)) 'Duplicate process-loss observation'
        $seen[$key] = $true
    }
    foreach ($repeat in 1..3) {
        foreach ($stage in $stages) {
            foreach ($inspection in 1..2) { Require $seen.ContainsKey("$repeat|$stage|true|$inspection") "Missing SIGKILL $repeat $stage $inspection" }
        }
        foreach ($inspection in 1..2) { Require $seen.ContainsKey("$repeat|AfterReturn|false|$inspection") 'Missing uninterrupted control' }
        Count $raw "^original_output_join_crash_summary seed=0x074051d1 repeat=$repeat sigkills=26 controls=1 fresh_inspections=54 production_enrollment=false start=false$" 1
    }
    Require ($raw.Contains('1 passed; 0 failed;')) 'Process-loss gate did not pass'
}
$m = Get-Content -LiteralPath (Join-Path $packet 'manifest.json') -Raw | ConvertFrom-Json
Scope $m.scope
$bad = $m.scope | ConvertTo-Json | ConvertFrom-Json
$bad.server_ready = $true
$rejected = $false
try { Scope $bad } catch { $rejected = $true }
Require $rejected 'False-admission canary survived'
Require ($m.baseline_commit -eq 'e06e2ceb83c3aeacbe5cd2aa6b541773e9e4c004') 'Wrong attribution baseline'
Require ($m.preregister_commit -eq 'a8f4d604d5a3e4548256b195a17b00022622b042') 'Wrong preregistration'
Require ($m.expiry_preregister_commit -eq 'cfa9eee3eb421ba4ffba0688cd870a75900f7060') 'Wrong expiry preregistration'
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
$actual = @(Get-ChildItem -LiteralPath $packet -File -Recurse | ForEach-Object { $_.FullName.Substring($packet.Length + 1).Replace('\','/') } | Where-Object { $_ -notin @('README.md','manifest.json','verify.ps1') } | Sort-Object)
Require (($actual -join '|') -eq (($m.artifacts.path | Sort-Object) -join '|')) 'Artifact inventory changed'
foreach ($snapshot in $m.source_snapshots) {
    $patch = [IO.File]::ReadAllText((Join-Path $packet $snapshot.patch)).Replace([string][char]13,'')
    $blocks = @([regex]::Split($patch, '(?m)(?=^diff --git )') | Where-Object { $_.StartsWith('diff --git ') })
    Require ($blocks.Count -eq $snapshot.paths) 'Wrong source snapshot inventory'
    $sourceSeen = @{}
    foreach ($block in $blocks) {
        $match = [regex]::Match($block, '(?m)^\+\+\+ b/(.+)$')
        Require $match.Success 'Missing source path'
        $path = $match.Groups[1].Value
        Require ($path -notmatch '(^/|^[A-Za-z]:|\\|\.\.)') 'Unsafe source path'
        Require (-not $sourceSeen.ContainsKey($path)) 'Duplicate source path'
        $sourceSeen[$path] = $true
        if ($block.Contains('--- /dev/null' + $lf)) { $old = @() } else {
            $old = @(& git -C $repo show ($m.preregister_commit + ':' + $path))
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
                    Require ($cursor -lt $old.Count -and [string]$old[$cursor] -ceq $body) "Old context differs $path at zero-based line $cursor"
                    $cursor++
                }
                if ($prefix -ne '-') { $output.Add($body) }
            }
        }
        while ($cursor -lt $old.Count) { $output.Add([string]$old[$cursor]); $cursor++ }
        $raw = [string]::Join($lf, $output.ToArray()) + $lf
        $hash = Blob $raw
        $index = [regex]::Match($block, '(?m)^index [a-f0-9]+\.\.([a-f0-9]+)')
        Require ($index.Success -and $hash.StartsWith($index.Groups[1].Value)) "Reconstructed source differs $path"
        foreach ($runtime in @($m.runtime_blobs | Where-Object { $_.path -ceq $path })) {
            Require ($hash -eq $runtime.git_blob) "Runtime differs between source cohorts $path"
        }
        if ($snapshot.active) {
            $live = [IO.File]::ReadAllText((Join-Path $repo $path)).Replace([string][char]13,'')
            Require ((Blob $live) -eq $hash) "Active source differs $path"
        }
    }
    foreach ($runtime in $m.runtime_blobs) { Require $sourceSeen.ContainsKey($runtime.path) 'Missing runtime source in cohort' }
}
$firstRed = [IO.File]::ReadAllText((Join-Path $packet 'j1-join-fault-194.log'))
Count $firstRed 'HC074-OUTPUT-JOIN-INTERRUPT cut=' 15
Count $firstRed 'HC074-OUTPUT-JOIN-LIVE seed=0x074051e1 cut=' 25
Require ($firstRed.Contains('0 passed; 1 failed;') -and $firstRed.Contains('Causal(Progress(Input(Kernel(Invalid))))')) 'First red oracle observation omitted'
$diagnostic = [IO.File]::ReadAllText((Join-Path $packet 'j1-expiry-attribution-194.log'))
Require ($diagnostic.Contains('site: "progress-age"') -and $diagnostic.Contains('1 passed; 0 failed;')) 'Fresh expiry attribution missing'
$times = [regex]::Match($diagnostic, 'clock: \[(\d+), (\d+), (\d+), 0, 0, (\d+)\]')
Require $times.Success 'Expiry clock record missing'
$received = [UInt64]$times.Groups[1].Value
$after = [UInt64]$times.Groups[2].Value
$now = [UInt64]$times.Groups[3].Value
$limit = [UInt64]$times.Groups[4].Value
Require ($received -gt $after -and $now -ge $received -and $limit -eq 10000000000 -and $now - $received -ge $limit) 'Invalid expiry attribution'
foreach ($b in $m.runtime_blobs) {
    Require ($b.path -notmatch '(^/|^[A-Za-z]:|\\|\.\.)') 'Unsafe J1 runtime path'
    $live = [IO.File]::ReadAllText((Join-Path $repo $b.path)).Replace([string][char]13,'')
    Require ((Blob $live) -eq $b.git_blob) "Runtime changed after attribution: $($b.path)"
}
foreach ($b in (Get-Content -LiteralPath (Join-Path $packet 'unchanged-runtime.json') -Raw | ConvertFrom-Json)) {
    Require ($b.path -notmatch '(^/|^[A-Za-z]:|\\|\.\.)') 'Unsafe original runtime path'
    $live = [IO.File]::ReadAllText((Join-Path $repo $b.path)).Replace([string][char]13,'')
    Require ((Blob $live) -eq $b.git_blob) "Original runtime changed: $($b.path)"
}
foreach ($lane in @('194','188')) {
    $baseline = [IO.File]::ReadAllText((Join-Path $packet "baseline-$lane.log"))
    Count $baseline 'HC074-OUTPUT-COST sample=[012] phase=starting full_frames=19 rounds=38' 3
    Count $baseline 'HC074-OUTPUT-COST sample=[012] phase=pipes full_frames=4 rounds=8' 3
    Count $baseline 'HC074-OUTPUT-LEAF sample=[012] step=ReservedLive ' 3
    Require ($baseline.Contains('complete=true samples=3 leaf_chains=3 renewed=false product=false') -and $baseline.Contains('1 passed; 0 failed;')) 'Baseline incomplete'
    $healthy = [IO.File]::ReadAllText((Join-Path $packet "j1-screen-$lane.log"))
    Count $healthy 'HC074-OUTPUT-JOIN-COST sample=[012] rounds=46 ' 3
    Count $healthy 'HC074-OUTPUT-JOIN-CONTROL sample=[012] ordered=25 empty=true drop_eof=true live_closing=true start=false' 3
    Count $healthy 'authentication_calls: 782,' 3
    $reference = [IO.File]::ReadAllText((Join-Path $packet "original-progress-expiry-$lane.log"))
    Require ($reference.Contains('real_expiry=true receive_time_unchanged=true deadline_extended=false') -and $reference.Contains('1 passed; 0 failed;')) 'Original expiry reference missing'
    $sinks = [IO.File]::ReadAllText((Join-Path $packet "j1v2-sink-fault-$lane.log"))
    Count $sinks 'HC074-SINK-INTERRUPT cut=' 10
    Count $sinks 'HC074-SINK-DRIFT kind=' 12
    Count $sinks 'HC074-SINK-ENTRY kind=' 4
    Count $sinks 'HC074-SINK-CONTROL ' 1
    Require ($sinks.Contains('1 passed; 0 failed;')) 'Sink fault gate incomplete'
    $join = [IO.File]::ReadAllText((Join-Path $packet "j1v2-join-fault-$lane.log"))
    Count $join 'HC074-OUTPUT-JOIN-INTERRUPT cut=' 15
    Count $join 'HC074-OUTPUT-JOIN-LIVE seed=0x074051e1 cut=' 25
    Count $join 'HC074-OUTPUT-JOIN-EXPIRY original_progress=true site=progress-age renewed=false created=false' 1
    Count $join 'HC074-OUTPUT-JOIN-EXPIRY-ATTRIBUTION event=' 1
    Require ($join.Contains('1 passed; 0 failed;')) 'Join live gate incomplete'
    $crash = [IO.File]::ReadAllText((Join-Path $packet "j1v2-crash-$lane.log"))
    Crash $crash
    $first = [regex]::Match($crash, '(?m)original_output_join_crash [^\r\n]+$').Value
    $rejected = $false
    try { Crash ($crash.Replace($first + $lf,'')) } catch { $rejected = $true }
    Require $rejected 'Missing inspection canary survived'
}
& git -C $repo diff --exit-code $m.baseline_commit -- docs/testing/performance/0.74/local-runs/original-collector-sinks-20261017 docs/testing/performance/0.74/qualification-manifest.toml
Require ($LASTEXITCODE -eq 0) 'Historical negative packet or qualification inputs changed'
Write-Output 'Original J1 output join proof intact: attribution, live matrices and 156 SIGKILL / 6 controls / 324 fresh inspections; original deadlines exact, production admission CLOSED'
