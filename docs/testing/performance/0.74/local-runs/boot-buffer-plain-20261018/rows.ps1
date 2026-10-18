function Read-HostRows([string]$Raw) {
    $plain='diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::tests::fence_preflight_tests::attempt_tests::collector_tests::observed_reservation_tests::collector_reserved_tests::progress_tests::start_consent_tests::validation_frame_tests::starting_tests::sink_tests::start_material_tests::pipe_transport_tests::safety_alarm_tests::leaf_cost_tests::component_cost_tests::live_walk_tests::plain_worker_tests::original_plain_worker_batch'
    $prepBase='diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::tests::fence_preflight_tests::attempt_tests::collector_tests::observed_reservation_tests::collector_reserved_tests::progress_tests::start_consent_tests::validation_frame_tests::starting_tests::sink_tests::start_material_tests::pipe_transport_tests::safety_alarm_tests::fixed_grouped_tests::custody_tests::descriptor_plan_tests::barred_custody_tests::collector_tests::enrollment_tests::manager_tests::addressed_tests::'
    $testNames=@('original_grouped_addressed_frame_controls','original_grouped_addressed_frame_refusals','original_grouped_addressed_nonroot_fixed_refusal')
    $expected=@(foreach ($compiler in @('194','188')) {
        foreach ($pair in 0..4) {
            $roles=if ($pair % 2 -eq 0) {@('Baseline','Candidate')} else {@('Candidate','Baseline')}
            foreach ($role in $roles) {"$compiler/$pair/$role"}
        }
    })
    $prepExpected=@(foreach ($compiler in @('194','188')) {foreach ($name in $testNames) {"$compiler/$name"}})
    $rows=@();$closed=@();$active=$null;$terminal=0
    $ages=@();$controls=@();$transfers=@();$refusals=@();$fixed=@()
    $cases=@('alias','client','flags-fd','body','swapped','serial','missing','channel-before','channel-after','context-before','context-after','message','output','custodian-kill','inside-message','inside-collector-kill','inside-final-expiry','trust','collector-kill','final-expiry')
    foreach ($line in ($Raw.Replace("`r",'') -split "`n")) {
        $value=$line
        if ($active -and $line.StartsWith('test ',[StringComparison]::Ordinal) -and $line.Contains('HC074-')) {
            $prefix="test $($active.name) ... "
            if (!$line.StartsWith($prefix,[StringComparison]::Ordinal)) {throw 'test prefix'}
            $value=$line.Substring($prefix.Length)
        }
        if ($value -cmatch '^HC074-PLAIN-INVOCATION compiler=(194|188) pair=([0-4]) role=(Baseline|Candidate) begin=true$') {
            $key="$($Matches[1])/$($Matches[2])/$($Matches[3])"
            if ($active -or $rows.Count -ge 20 -or $key -cne $expected[$rows.Count]) {throw 'plain order'}
            $active=@{kind='plain';key=$key;compiler=$Matches[1];pair=[int]$Matches[2];role=$Matches[3];name=$plain;wall=$null;passed=$false}
        } elseif ($value -cmatch '^HC074-PLAIN-WORKER iterations=128 wall_ns=([1-9][0-9]{0,18}) original=true unchanged=true capture=false reservation=false start=false$') {
            if (!$active -or $active.kind -cne 'plain' -or $null -ne $active.wall) {throw 'plain row context'}
            $active.wall=[UInt64]$Matches[1]
        } elseif ($value.Contains('HC074-PLAIN-WORKER')) {throw 'plain row grammar'
        } elseif ($value -cmatch '^HC074-PLAIN-INVOCATION compiler=(194|188) pair=([0-4]) role=(Baseline|Candidate) completed=true$') {
            $key="$($Matches[1])/$($Matches[2])/$($Matches[3])"
            if (!$active -or $active.kind -cne 'plain' -or $key -cne $active.key -or !$active.passed -or $null -eq $active.wall) {throw 'plain completion'}
            $rows+=@{compiler=$active.compiler;pair=$active.pair;role=$active.role;iterations=128;wall_ns=$active.wall}
            $active=$null
        } elseif ($value -cmatch '^HC074-PLAIN-PREPARATION compiler=(194|188) test=([a-z_]+) begin=true$') {
            $key="$($Matches[1])/$($Matches[2])"
            if ($active -or $rows.Count -ne 20 -or $closed.Count -ge 6 -or $key -cne $prepExpected[$closed.Count]) {throw 'preparation order'}
            $active=@{kind='preparation';key=$key;compiler=$Matches[1];name=$prepBase+$Matches[2];passed=$false}
        } elseif ($value -cmatch '^HC074-PLAIN-PREPARATION compiler=(194|188) test=([a-z_]+) completed=true$') {
            $key="$($Matches[1])/$($Matches[2])"
            if (!$active -or $active.kind -cne 'preparation' -or $key -cne $active.key -or !$active.passed) {throw 'preparation completion'}
            $closed+=$key;$active=$null
        } elseif ($value -cmatch '^test result: ok\. 1 passed; 0 failed; 0 ignored;') {
            if (!$active -or $active.passed) {throw 'suite context'}
            $active.passed=$true
        } elseif ($value -cmatch '^HC074-GROUPED-AGE sample=([0-2]) pass=([01]) issued_ns=([0-9]+) age_before_ns=([0-9]+) age_after_ns=([0-9]+) auth=34 signatures=34 channel_checks=2 result=Ok\(\(\)\)$') {
            if (!$active -or $active.kind -cne 'preparation' -or !$active.name.EndsWith($testNames[0],[StringComparison]::Ordinal)) {throw 'age context'}
            $before=[UInt64]$Matches[4];$after=[UInt64]$Matches[5]
            if ($after -lt $before -or $after -ge 10000000000) {throw 'original deadline'}
            $ages+=@{compiler=$active.compiler;sample=[int]$Matches[1];pass=[int]$Matches[2];issued_ns=[UInt64]$Matches[3];age_before_ns=$before;age_after_ns=$after}
        } elseif ($value -cmatch '^HC074-GROUPED-ADDRESSED-CONTROL sample=([0-2]) pass=([01]) frame_ipc=34 channel_checks=2 test_channel_only=true start=false$') {
            if (!$active -or !$active.name.EndsWith($testNames[0],[StringComparison]::Ordinal)) {throw 'control context'}
            $controls+="$($active.compiler)/$($Matches[1])/$($Matches[2])"
        } elseif ($value -cmatch '^HC074-GROUPED-ADDRESSED-TRANSFER sample=([0-2]) original_writers=2 rights=true same_process_socket=true system_bus_delivery=false start=false$') {
            if (!$active -or !$active.name.EndsWith($testNames[0],[StringComparison]::Ordinal)) {throw 'transfer context'}
            $transfers+="$($active.compiler)/$($Matches[1])"
        } elseif ($value -cmatch '^HC074-GROUPED-ADDRESSED-REFUSAL case=([^ ]+) reason=(.+) sticky=true start=false$') {
            if (!$active -or !$active.name.EndsWith($testNames[1],[StringComparison]::Ordinal) -or $Matches[1] -cnotin $cases -or $Matches[2].Contains('Time')) {throw 'refusal context'}
            $refusals+="$($active.compiler)/$($Matches[1])"
        } elseif ($value -ceq 'HC074-GROUPED-ADDRESSED nonroot=true fixed_refused=true original_owner_retained=true start=false') {
            if (!$active -or !$active.name.EndsWith($testNames[2],[StringComparison]::Ordinal)) {throw 'fixed context'}
            $fixed+=$active.compiler
        } elseif ($value -ceq 'HC074-PLAIN-HOST completed=true supervisor_unchanged=true no_fixture_executable=true start=false') {
            if ($active) {throw 'unfinished invocation'};$terminal++
        } elseif ($value.Contains('HC074-GROUPED-') -or $value.Contains('HC074-PLAIN-INVOCATION') -or $value.Contains('HC074-PLAIN-PREPARATION')) {throw 'unexpected diagnostic row'}
    }
    if ($active -or $rows.Count -ne 20 -or $closed.Count -ne 6 -or $terminal -ne 1) {throw 'host inventory'}
    $ageKeys=@($ages | ForEach-Object {"$($_.compiler)/$($_.sample)/$($_.pass)"})
    $rounds=@(foreach ($c in @('194','188')) {foreach ($s in 0..2) {foreach ($p in 0..1) {"$c/$s/$p"}}})
    if (($ageKeys -join ',') -cne ($rounds -join ',') -or ($controls -join ',') -cne ($rounds -join ',')) {throw 'round inventory'}
    $wantedTransfers=@(foreach ($c in @('194','188')) {foreach ($s in 0..2) {"$c/$s"}})
    $wantedRefusals=@(foreach ($c in @('194','188')) {foreach ($case in $cases) {"$c/$case"}})
    if (($transfers -join ',') -cne ($wantedTransfers -join ',') -or ($refusals -join ',') -cne ($wantedRefusals -join ',') -or ($fixed -join ',') -cne '194,188') {throw 'fault/transfer inventory'}
    return @{rows=$rows;ages=$ages;healthy_rounds=12;refusals=40;transfers=6;fixed_refusals=2}
}
function Paired-Statistics($Rows) {
    $cohorts=@()
    foreach ($compiler in @('194','188')) {
        $pairs=@();$logs=@()
        foreach ($pair in 0..4) {
            $a=@($Rows | Where-Object {$_.compiler -ceq $compiler -and $_.pair -eq $pair -and $_.role -ceq 'Baseline'})
            $b=@($Rows | Where-Object {$_.compiler -ceq $compiler -and $_.pair -eq $pair -and $_.role -ceq 'Candidate'})
            if ($a.Count -ne 1 -or $b.Count -ne 1) {throw 'paired inventory'}
            $ratio=[double]$b[0].wall_ns/[double]$a[0].wall_ns
            $logs+=[Math]::Log($ratio)
            $pairs+=@{pair=$pair;baseline_ns_per_validation=([double]$a[0].wall_ns/128);candidate_ns_per_validation=([double]$b[0].wall_ns/128);ratio=$ratio}
        }
        $mean=($logs | Measure-Object -Average).Average
        $variance=0.0
        foreach ($value in $logs) {$variance+=[Math]::Pow($value-$mean,2)}
        $half=2.7764451051977987*[Math]::Sqrt($variance/4)/[Math]::Sqrt(5)
        $lower=[Math]::Exp($mean-$half);$upper=[Math]::Exp($mean+$half)
        $cohorts+=@{compiler=$compiler;pairs=$pairs;geometric_mean_ratio=[Math]::Exp($mean);ci95_lower=$lower;ci95_upper=$upper;lower_cost_confirmed=($upper -le 0.99);non_regression=($upper -le 1.03);product_claim=$false}
    }
    return ,$cohorts
}
