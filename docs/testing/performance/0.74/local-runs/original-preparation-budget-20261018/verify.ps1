param([switch]$SelfTest)
$ErrorActionPreference='Stop'
$root=(Resolve-Path (Join-Path $PSScriptRoot '../../../../../..')).Path
function Require([bool]$Value,[string]$Reason){if(!$Value){throw $Reason}}
function Raw([string]$Path){[IO.File]::ReadAllText((Join-Path $PSScriptRoot $Path))}
function HashText([string]$Value){[Convert]::ToHexString([Security.Cryptography.SHA256]::HashData([Text.Encoding]::UTF8.GetBytes($Value))).ToLowerInvariant()}
function GitText([string]$Ref,[string]$Path){
    $start=[Diagnostics.ProcessStartInfo]::new('git');$start.WorkingDirectory=$root;$start.UseShellExecute=$false;$start.RedirectStandardOutput=$true;$start.StandardOutputEncoding=[Text.UTF8Encoding]::new($false)
    $start.ArgumentList.Add('show');$start.ArgumentList.Add("${Ref}:$Path")
    $process=[Diagnostics.Process]::Start($start);$value=$process.StandardOutput.ReadToEnd();$process.WaitForExit();Require ($process.ExitCode -eq 0) 'git source';$value.Replace("`r`n","`n")
}
function Pass([string]$Log,[int]$Count){Require ($Log -cnotmatch 'FAILED|panicked|error:|Exception:' -and $Log.Contains("test result: ok. $Count passed; 0 failed;")) 'passing gate'}
function HostPass([string]$Log){
    Pass $Log 1
    Require ([regex]::Matches($Log,'test result: ok. 1 passed; 0 failed;').Count -eq 2) 'host exact test counts'
    Require ([regex]::Matches($Log,'HC074-PREPARATION-CONTROL sample=').Count -eq 6) 'diagnostic controls'
    Require ([regex]::Matches($Log,'HC074-PREPARATION-COST sample=').Count -eq 48) 'attribution rows'
    Require ([regex]::Matches($Log,'HC074-PREPARATION-POINT sample=').Count -eq 48) 'original landmarks'
    Require ([regex]::Matches($Log,'HC074-PREPARATION-ROUND sample=[0-2] pass=[01] channel_checks=2 stats=Stats \{ wall_ns: [0-9]+, authentication_calls: 34, authentication_ns: [0-9]+, signature_calls: 34,').Count -eq 12) 'full unchanged rounds'
    foreach($compiler in @('194','188')){Require ($Log.Contains("HC074-PREPARATION-HOST compiler=$compiler completed=true start=false")) 'compiler host completion'}
    Require ($Log.Contains('HC074-PREPARATION-HOST completed=true supervisor_unchanged=true no_fixture_executable=true admission=false')) 'final host guard'
}
function Observations($data,[string]$Log){
    Require ($data.kind -ceq 'correctness-attribution' -and $data.product_performance_claim -ceq $false -and $data.observation_schedule_changed -ceq $false) 'numeric scope'
    Require ($data.rows.Count -eq 48 -and $data.points.Count -eq 48 -and $data.rounds.Count -eq 12) 'numeric inventory'
    $parts=[regex]::Split($Log,'HC074-PREPARATION-HOST compiler=(194|188) begin=true start=false')
    Require ($parts.Count -eq 5 -and $parts[1] -ceq '194' -and $parts[3] -ceq '188') 'owned compiler sections'
    $logs=@{'194'=$parts[2];'188'=$parts[4]}
    foreach($group in @(@{rows=$data.rows;kind='COST';key='bucket'},@{rows=$data.rounds;kind='ROUND';key='pass'})){
        foreach($row in $group.rows){
            Require ($row.compiler -cin @('194','188') -and $row.sample -in @(0,1,2)) 'numeric owner'
            $stats="Stats { wall_ns: $($row.wall_ns), authentication_calls: $($row.authentication_calls), authentication_ns: $($row.authentication_ns), signature_calls: $($row.signature_calls), signature_ns: $($row.signature_ns), worker_policy_calls: $($row.worker_policy_calls), worker_policy_ns: $($row.worker_policy_ns) }"
            $selector=if($group.kind -ceq 'COST'){"bucket=$($row.bucket)"}else{"pass=$($row.pass) channel_checks=2"}
            Require ($logs[$row.compiler].Contains("HC074-PREPARATION-$($group.kind) sample=$($row.sample) $selector stats=$stats")) 'raw numeric binding'
        }
    }
    foreach($row in $data.points){
        Require ($logs[$row.compiler].Contains("HC074-PREPARATION-POINT sample=$($row.sample) point=$($row.point) issued_ns=$($row.issued_ns) now_ns=$($row.now_ns) age_ns=$($row.age_ns)")) 'raw landmark binding'
        Require ($row.now_ns - $row.issued_ns -eq $row.age_ns -and $row.age_ns -lt 10000000000) 'original freshness'
    }
    foreach($compiler in @('194','188')){foreach($sample in 0..2){
        $owned=@($data.rows | Where-Object {$_.compiler -ceq $compiler -and $_.sample -eq $sample})
        Require ((($owned.bucket | Sort-Object) -join ',') -ceq ((@('prechallenge-reservation','challenge-creation','response-acceptance','consent-preparation','consent-acceptance','joined-outputs','enrollment','manager-bookkeeping-inclusive') | Sort-Object) -join ',')) 'exact buckets'
        $rounds=@($data.rounds | Where-Object {$_.compiler -ceq $compiler -and $_.sample -eq $sample})
        Require (($rounds.pass -join ',') -ceq '0,1') 'two original rounds'
        $points=@($data.points | Where-Object {$_.compiler -ceq $compiler -and $_.sample -eq $sample})
        Require (($points.point -join ',') -ceq 'challenge-ready,response-ready,consent-prepared,consent-ready,outputs-ready,enrollment-ready,manager-prepare-ready,manager-delivery-ready') 'exact landmarks'
        Require (@($points.issued_ns | Sort-Object -Unique).Count -eq 1) 'immutable issued'
        $last=0;foreach($point in $points){Require ($point.now_ns -ge $last) 'landmark order';$last=$point.now_ns}
    }}
}
$flags=@('admission','original_start','writer_cessation','server_install','supervisor_changed','product_workload','negotiated_bus_delivery','positive_production_join')
function Scope($Manifest){
    Require ($Manifest.schema -ceq 'hc074-original-preparation-budget-v1') 'schema'
    Require ($Manifest.base_commit -ceq '537e1f253570bdff4e5f07316701a12d4c172da3') 'baseline'
    foreach($flag in $flags){Require ($Manifest.$flag -is [bool] -and $Manifest.$flag -ceq $false) "scope $flag"}
    Require ($Manifest.source_files.Count -eq 5 -and $Manifest.build_inputs.Count -eq 352 -and $Manifest.images.Count -eq 2) 'source inventory'
    Require ($Manifest.qualification_sha256 -ceq '11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc') 'qualification'
    Require ($Manifest.status -cin @('accepted-diagnostic','stopped-diagnostic')) 'status'
    Require ($Manifest.host_attribution_complete -is [bool]) 'host flag type'
    if($Manifest.status -ceq 'accepted-diagnostic'){
        Require ($Manifest.host_attribution_complete -ceq $true -and $Manifest.source_commit -cmatch '^[a-f0-9]{40}$') 'accepted fixture identity'
    }else{
        Require ($Manifest.host_attribution_complete -ceq $false -and $null -eq $Manifest.source_commit) 'withdrawn runtime identity'
    }
}
$m=Raw 'manifest.json' | ConvertFrom-Json
Scope $m
$actual=@(Get-ChildItem -LiteralPath $PSScriptRoot -Recurse -File | ForEach-Object {[IO.Path]::GetRelativePath($PSScriptRoot,$_.FullName).Replace('\','/')} | Where-Object {$_ -cne 'manifest.json'} | Sort-Object)
Require (($actual -join "`n") -ceq (($m.files.path | Sort-Object) -join "`n")) 'closed evidence inventory'
foreach($entry in $m.files){Require ($entry.path -cnotmatch '(^/|\\|\.\.)') 'unsafe path';Require ((Get-FileHash -LiteralPath (Join-Path $PSScriptRoot $entry.path) -Algorithm SHA256).Hash.ToLowerInvariant() -ceq $entry.sha256) "file $($entry.path)"}
foreach($entry in $m.source_files){
    $capsule='source-capsule/'+$entry.path.Replace('/','__')+'.source'
    Require ((Get-FileHash -LiteralPath (Join-Path $PSScriptRoot $capsule) -Algorithm SHA256).Hash.ToLowerInvariant() -ceq $entry.raw_sha256) 'raw source capsule'
    Require ((HashText ((Raw $capsule).Replace("`r`n","`n"))) -ceq $entry.lf_sha256) 'LF source capsule'
    if($m.source_commit){Require ((GitText $m.source_commit $entry.path) -ceq (Raw $capsule).Replace("`r`n","`n")) 'committed source'}
}
$expected=@(git -C $root ls-tree -r --name-only $m.base_commit -- tools/long-run-supervisor-074/src | Where-Object {$_ -cmatch '^tools/long-run-supervisor-074/src/.+\.rs$'})
$expected+=@($m.source_files.path | Where-Object {$_ -cmatch '^tools/long-run-supervisor-074/src/[^/]+\.rs$'})
$expected+=@('Cargo.toml','Cargo.lock','tools/long-run-supervisor-074/Cargo.toml')
Require ((($expected | Sort-Object -Unique) -join "`n") -ceq (($m.build_inputs.path | Sort-Object) -join "`n")) 'complete build inventory'
foreach($entry in $m.build_inputs){
    $changed=@($m.source_files | Where-Object {$_.path -ceq $entry.path})
    $hash=if($changed.Count -eq 1){$changed[0].lf_sha256}else{HashText (GitText $m.base_commit $entry.path)}
    Require ($hash -ceq $entry.lf_sha256) 'selected build input'
}
Require ((($m.images.compiler | Sort-Object) -join ',') -ceq '188,194') 'compiler identity'
foreach($image in $m.images){
    Require ($image.sha256 -cmatch '^[a-f0-9]{64}$' -and $image.image -cmatch "^/home/artur/\.cache/hydracache-074-$($image.compiler)-check/release/deps/hydracache_long_run_supervisor_074-[a-f0-9]{16}$") 'selected image shape'
    $receipt=@((Raw 'Candidate-images.json' | ConvertFrom-Json) | Where-Object {$_.compiler -ceq $image.compiler})
    Require ($receipt.Count -eq 1 -and $receipt[0].sha256 -ceq $image.sha256 -and $receipt[0].image -ceq $image.image -and $receipt[0].version -ceq $image.version) 'image receipt'
    Require ((Raw 'host.log').Contains($image.sha256+"  ./tests-$($image.compiler)")) 'host image binding'
    $build=@((Raw "Candidate-build-$($image.compiler).jsonl") -split "`n" | Where-Object {$_.StartsWith('{')} | ForEach-Object {$_ | ConvertFrom-Json})
    $artifact=@($build | Where-Object {$_.reason -ceq 'compiler-artifact' -and $_.target.name -ceq 'hydracache_long_run_supervisor_074' -and $_.profile.test -ceq $true -and $_.executable})
    Require ($artifact.Count -eq 1 -and $artifact[0].executable -ceq $image.image) 'compiler artifact'
    Require (@($build | Where-Object {$_.reason -ceq 'build-finished' -and $_.success -ceq $true}).Count -eq 1) 'successful image build'
    Require ((Raw "Candidate-$($image.compiler).log").Contains("image=$($image.sha256) compiler=$($image.compiler)")) 'unit image binding'
    foreach($count in @(15,19,5)){Pass (Raw "Candidate-$($image.compiler).log") $count}
    foreach($count in @(15,19,5)){Pass (Raw "Baseline-$($image.compiler).log") $count}
    Require ((Raw "gnu-$($image.compiler).log").Contains("compiler=$($image.compiler) complete=true admission=false")) 'GNU check and strict Clippy'
}
Pass (Raw 'windows-tests.log') 44
Pass (Raw 'contracts.log') 243
foreach($compiler in @('194','188')){Pass (Raw "trace-controls-$compiler.log") 6}
Require ((Raw 'host-postflight.log').Contains('preflight=ok supervisor_pid=8839 restarts=0')) 'host postflight'
if($m.status -ceq 'accepted-diagnostic'){
    HostPass (Raw 'host.log')
    Observations (Raw 'observations.json' | ConvertFrom-Json) (Raw 'host.log')
    $parent=(git -C $root rev-parse "$($m.source_commit)^").Trim()
    Require ($LASTEXITCODE -eq 0 -and $parent -ceq $m.base_commit) 'source parent'
    $changes=@(git -C $root diff --name-only $m.base_commit $m.source_commit)
    Require ((($changes | Sort-Object) -join "`n") -ceq (($m.source_files.path | Sort-Object) -join "`n")) 'exact source change set'
}else{
    Require ((Raw 'host.log').Contains('FAILED') -and (Raw 'host.log').Contains('Causal(Time)')) 'retained stopped diagnostic cohort'
    Require (!(Raw 'host.log').Contains('HC074-PREPARATION-HOST completed=true')) 'no promotion of stopped cohort'
}
if($SelfTest){
    $count=0
    foreach($flag in $flags){$copy=(Raw 'manifest.json') | ConvertFrom-Json;$copy.$flag=$true;$failed=$false;try{Scope $copy}catch{$failed=$true};Require $failed "negative scope $flag";$count++}
    $copy=(Raw 'manifest.json') | ConvertFrom-Json;$copy.qualification_sha256='0'*64;$failed=$false;try{Scope $copy}catch{$failed=$true};Require $failed 'negative qualification';$count++
    foreach($log in @('',(Raw 'host.log').Replace('authentication_calls: 34','authentication_calls: 0'),(Raw 'host.log').Replace('HC074-PREPARATION-CONTROL sample=','removed-control sample='))){$failed=$false;try{HostPass $log}catch{$failed=$true};Require $failed 'negative host evidence';$count++}
    foreach($case in @('claim','point','age','bucket')){
        $copy=(Raw 'observations.json') | ConvertFrom-Json
        switch($case){'claim'{$copy.product_performance_claim=$true};'point'{$copy.points[1]=$copy.points[0]};'age'{$copy.points[0].age_ns=10000000000};'bucket'{$copy.rows[0].bucket='wrong'}}
        $failed=$false;try{Observations $copy (Raw 'host.log')}catch{$failed=$true}
        Require $failed "negative numeric $case";$count++
    }
    "HC074-PREPARATION-VERIFY negative_controls=$count passed=true"
}
"HC074-PREPARATION-VERIFY status=$($m.status) sources=5 inputs=352 admission=false start=false"
