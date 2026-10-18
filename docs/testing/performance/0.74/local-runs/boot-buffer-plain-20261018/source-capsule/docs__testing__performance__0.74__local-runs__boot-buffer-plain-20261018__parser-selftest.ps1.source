$ErrorActionPreference='Stop'
. (Join-Path $PSScriptRoot 'rows.ps1')
$lines=[Collections.Generic.List[string]]::new()
foreach ($c in @('194','188')) {
    foreach ($p in 0..4) {
        $roles=if ($p % 2 -eq 0) {@('Baseline','Candidate')} else {@('Candidate','Baseline')}
        foreach ($role in $roles) {
            $wall=if ($role -ceq 'Baseline') {1280000} else {1152000}
            $lines.Add("HC074-PLAIN-INVOCATION compiler=$c pair=$p role=$role begin=true")
            $lines.Add("HC074-PLAIN-WORKER iterations=128 wall_ns=$wall original=true unchanged=true capture=false reservation=false start=false")
            $lines.Add('test result: ok. 1 passed; 0 failed; 0 ignored;')
            $lines.Add("HC074-PLAIN-INVOCATION compiler=$c pair=$p role=$role completed=true")
        }
    }
}
foreach ($c in @('194','188')) {
    foreach ($name in @('original_grouped_addressed_frame_controls','original_grouped_addressed_frame_refusals','original_grouped_addressed_nonroot_fixed_refusal')) {
        $lines.Add("HC074-PLAIN-PREPARATION compiler=$c test=$name begin=true")
        if ($name.EndsWith('_controls',[StringComparison]::Ordinal)) {
            foreach ($s in 0..2) {
                foreach ($p in 0..1) {
                    $lines.Add("HC074-GROUPED-AGE sample=$s pass=$p issued_ns=1 age_before_ns=10 age_after_ns=20 auth=34 signatures=34 channel_checks=2 result=Ok(())")
                    $lines.Add("HC074-GROUPED-ADDRESSED-CONTROL sample=$s pass=$p frame_ipc=34 channel_checks=2 test_channel_only=true start=false")
                }
                $lines.Add("HC074-GROUPED-ADDRESSED-TRANSFER sample=$s original_writers=2 rights=true same_process_socket=true system_bus_delivery=false start=false")
            }
        } elseif ($name.EndsWith('_refusals',[StringComparison]::Ordinal)) {
            foreach ($case in @('alias','client','flags-fd','body','swapped','serial','missing','channel-before','channel-after','context-before','context-after','message','output','custodian-kill','inside-message','inside-collector-kill','inside-final-expiry','trust','collector-kill','final-expiry')) {
                $lines.Add("HC074-GROUPED-ADDRESSED-REFUSAL case=$case reason=Frame(Message) sticky=true start=false")
            }
        } else {$lines.Add('HC074-GROUPED-ADDRESSED nonroot=true fixed_refused=true original_owner_retained=true start=false')}
        $lines.Add('test result: ok. 1 passed; 0 failed; 0 ignored;')
        $lines.Add("HC074-PLAIN-PREPARATION compiler=$c test=$name completed=true")
    }
}
$lines.Add('HC074-PLAIN-HOST completed=true supervisor_unchanged=true no_fixture_executable=true start=false')
$raw=$lines -join "`n"
$parsed=Read-HostRows $raw
$stats=Paired-Statistics $parsed.rows
foreach ($cohort in $stats) {
    if ([Math]::Abs($cohort.geometric_mean_ratio-0.9) -gt 1e-12 -or [Math]::Abs($cohort.ci95_lower-0.9) -gt 1e-12 -or [Math]::Abs($cohort.ci95_upper-0.9) -gt 1e-12 -or !$cohort.non_regression -or !$cohort.lower_cost_confirmed) {throw 'synthetic estimator control'}
}
$first='HC074-PLAIN-WORKER iterations=128 wall_ns=1280000 original=true unchanged=true capture=false reservation=false start=false'
$bad=@(
 $raw.Replace('iterations=128','iterations=127'),
 $raw.Replace($first,"$first`n$first"),
 $raw.Replace('compiler=194 pair=0 role=Baseline begin=true','compiler=194 pair=0 role=Candidate begin=true'),
 $raw.Replace('HC074-PLAIN-INVOCATION compiler=194 pair=0 role=Baseline completed=true',''),
 $raw.Replace('wall_ns=1280000','wall_ns=NaN'),
 $raw.Replace('HC074-GROUPED-ADDRESSED-CONTROL sample=0 pass=0 frame_ipc=34 channel_checks=2 test_channel_only=true start=false',''),
 $raw.Replace('HC074-GROUPED-ADDRESSED-REFUSAL case=alias reason=Frame(Message) sticky=true start=false',''),
 $raw.Replace('age_after_ns=20','age_after_ns=10000000000'),
 $raw.Replace('capture=false','capture=true')
)
foreach ($value in $bad) {
    $refused=$false
    try {Read-HostRows $value | Out-Null} catch {$refused=$true}
    if (!$refused) {throw 'negative parser control accepted'}
}
'HC074-PLAIN-PARSER synthetic_only=true constant_ratio=0.9 negative_controls=9 refused=true'
