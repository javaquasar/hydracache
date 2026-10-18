param([switch]$SelfTest)
$ErrorActionPreference='Stop'
$root=(Resolve-Path (Join-Path $PSScriptRoot '../../../../../..')).Path
function Require([bool]$v,[string]$reason){if(!$v){throw $reason}}
function Raw([string]$path){[IO.File]::ReadAllText((Join-Path $PSScriptRoot $path))}
function HashText([string]$v){[Convert]::ToHexString([Security.Cryptography.SHA256]::HashData([Text.Encoding]::UTF8.GetBytes($v))).ToLowerInvariant()}
function GitText([string]$path){
    $s=[Diagnostics.ProcessStartInfo]::new('git');$s.WorkingDirectory=$root;$s.UseShellExecute=$false;$s.RedirectStandardOutput=$true;$s.StandardOutputEncoding=[Text.UTF8Encoding]::new($false)
    $s.ArgumentList.Add('show');$s.ArgumentList.Add("$($m.base_commit):$path")
    $p=[Diagnostics.Process]::Start($s);$v=$p.StandardOutput.ReadToEnd();$p.WaitForExit();Require ($p.ExitCode -eq 0) 'base input';$v.Replace("`r`n","`n")
}
$flags=@('admission','original_start','writer_cessation','server_install','supervisor_changed','product_workload','negotiated_bus_delivery','positive_production_join','host_fixture_cohort_pass','candidate_promoted')
function Scope($m){
    Require ($m.schema -ceq 'hc074-original-manager-message-rejected-v1' -and $m.status -ceq 'withdrawn-deadline' -and $null -eq $m.source_commit) 'rejected identity'
    foreach($flag in $flags){Require ($m.$flag -is [bool] -and $m.$flag -ceq $false) "scope $flag"}
    Require ($m.base_commit -ceq '959456253a74a6c0a2e9dfee9a7615f447af7448' -and $m.source_files.Count -eq 14 -and $m.build_inputs.Count -eq 346) 'source inventory'
    Require ($m.images.Count -eq 2 -and (($m.images.compiler | Sort-Object) -join ',') -ceq '188,194') 'images'
    Require ($m.qualification_sha256 -ceq '11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc') 'qualification'
}
function Pass([string]$log,[int]$count){Require ($log -cnotmatch 'FAILED|panicked|error:|Exception:' -and $log.Contains("test result: ok. $count passed; 0 failed;")) 'passing log'}
function HostPass([string]$log){Pass $log 1;Require ($log.Contains('HC074-ADDRESSED-HOST completed=true supervisor_unchanged=true no_fixture_executable=true admission=false')) 'not a completed host gate'}
$m=Raw 'manifest.json' | ConvertFrom-Json
Scope $m
$actual=@(Get-ChildItem -LiteralPath $PSScriptRoot -Recurse -File | ForEach-Object {[IO.Path]::GetRelativePath($PSScriptRoot,$_.FullName).Replace('\','/')} | Where-Object {$_ -cne 'manifest.json'} | Sort-Object)
Require (($actual -join "`n") -ceq (($m.files.path | Sort-Object) -join "`n")) 'closed inventory'
foreach($entry in $m.files){Require ($entry.path -cnotmatch '(^/|\\|\.\.)') 'unsafe path';Require ((Get-FileHash -LiteralPath (Join-Path $PSScriptRoot $entry.path) -Algorithm SHA256).Hash.ToLowerInvariant() -ceq $entry.sha256) "hash $($entry.path)"}
foreach($entry in $m.source_files){
    $path='source-capsule/'+$entry.path.Replace('/','__')+'.source'
    Require ((Get-FileHash -LiteralPath (Join-Path $PSScriptRoot $path) -Algorithm SHA256).Hash.ToLowerInvariant() -ceq $entry.raw_sha256) 'raw capsule'
    Require ((HashText ((Raw $path).Replace("`r`n","`n"))) -ceq $entry.lf_sha256) 'LF capsule'
}
$expected=@(git -C $root ls-tree -r --name-only $m.base_commit -- tools/long-run-supervisor-074/src | Where-Object {$_ -cmatch '^tools/long-run-supervisor-074/src/[^/]+\.rs$'})
$expected+=@($m.source_files.path | Where-Object {$_ -cmatch '^tools/long-run-supervisor-074/src/[^/]+\.rs$'})
$expected+=@('Cargo.toml','Cargo.lock','tools/long-run-supervisor-074/Cargo.toml')
Require ((($expected | Sort-Object -Unique) -join "`n") -ceq (($m.build_inputs.path | Sort-Object) -join "`n")) 'complete selected build inputs'
foreach($entry in $m.build_inputs){
    $changed=@($m.source_files | Where-Object {$_.path -ceq $entry.path})
    $hash=if($changed.Count -eq 1){$changed[0].lf_sha256}else{HashText (GitText $entry.path)}
    Require ($hash -ceq $entry.lf_sha256) 'selected overlay binding'
}
foreach($entry in $m.images){
    Require ((Raw 'host.log').Contains($entry.sha256+"  ./tests-$($entry.compiler)")) 'selected host image'
    Require ((Raw "Candidate-build-$($entry.compiler).jsonl").Contains($entry.image)) 'compiler artifact'
    foreach($n in @(14,19,5)){Pass (Raw "Candidate-$($entry.compiler).log") $n}
    foreach($n in @(10,19,5)){Pass (Raw "Baseline-$($entry.compiler).log") $n}
    Require ((Raw "gnu-$($entry.compiler).log").Contains("compiler=$($entry.compiler) complete=true admission=false")) 'candidate checks/lint'
}
$hostlog=Raw 'host.log'
Require ($hostlog.Contains('pass=0 frame_ipc=68 channel_checks=4') -and $hostlog.Contains('Causal(Time)') -and $hostlog.Contains('FAILED')) 'selected stopped cohort'
Require (!$hostlog.Contains('HC074-ADDRESSED-HOST compiler=194 completed=true') -and !$hostlog.Contains('HC074-ADDRESSED-TRANSFER sample=')) 'no later accepted host cases'
Require ((Raw 'host-postflight.log').Contains('preflight=ok supervisor_pid=8839 restarts=0')) 'selected postflight'
$a=Raw 'attribution.log'
Require ($a.Contains($m.attribution_image.sha256+'  ./tests-attribution-194')) 'diagnostic image'
Require ($a.Contains('auth=68 signatures=68 channel_checks=4 result=Ok(())') -and $a.Contains('auth=17 signatures=17 channel_checks=1 result=Err(')) 'diagnostic counters'
Require ($a.Contains('age_before_ns=9301862561 age_after_ns=9976507282') -and $a.Contains('age_before_ns=9976517902 age_after_ns=10116451429')) 'immutable challenge age'
Require ((Raw 'attribution-postflight.log').Contains('preflight=ok supervisor_pid=8839 restarts=0')) 'diagnostic postflight'
$original=(Raw 'source-capsule/tools__long-run-supervisor-074__src__diagnostic_original_manager_message_frame_tests.rs.source').Replace("`r`n","`n")
$diagnostic=(Raw 'attribution-frame.source').Replace("`r`n","`n")
$marker='#[ignore = "manual nonroot original addressed message full-frame and owned rights controls"]'
Require ($diagnostic.Substring($diagnostic.IndexOf($marker)) -ceq $original.Substring($original.IndexOf($marker))) 'diagnostic retains all old selected tests'
Pass (Raw 'contracts.log') 241;Pass (Raw 'windows-tests.log') 44
Require ((Raw 'Red-build-194.jsonl').Contains('E0599') -and (Raw 'Red-build-194.jsonl').Contains('prepare_original_message')) 'expected missing constructor'
Require ((Raw 'approved-final.log').Contains('HC074-ADDRESSED-WITHDRAWAL runtime_matches_base=true admission=false')) 'approved source restoration'
if($SelfTest){
    $count=0
    foreach($flag in $flags){$bad=$m|ConvertTo-Json -Depth 12|ConvertFrom-Json;$bad.$flag=$true;$rejected=$false;try{Scope $bad}catch{$rejected=$true};Require $rejected "negative $flag";$count++}
    $bad=$m|ConvertTo-Json -Depth 12|ConvertFrom-Json;$bad.build_inputs=@($bad.build_inputs | Select-Object -Skip 1);$rejected=$false;try{Scope $bad}catch{$rejected=$true};Require $rejected 'missing input';$count++
    foreach($log in @($hostlog,$a)){$rejected=$false;try{HostPass $log}catch{$rejected=$true};Require $rejected 'stopped/diagnostic promoted';$count++}
    "HC074-ADDRESSED-REJECTED-OFFLINE negative_controls=$count rejected=true"
}
'HC074-ADDRESSED-REJECTED-OFFLINE verified=true promoted=false read_only=true'
