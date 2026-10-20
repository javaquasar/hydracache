$ErrorActionPreference='Stop'
function NativeFacts {
    $taskRows=@()
    foreach($taskLane in '194','188'){
        $taskRaw=Get-Content -Raw "$PSScriptRoot/remote/$taskLane-matrix.log"
        $taskReports=@([regex]::Matches($taskRaw,'HC074-HELD-LIVE-ORIGINAL ([^\r\n]+)')|ForEach-Object{$_.Groups[1].Value|ConvertFrom-Json})
        if($taskReports.Count -ne 1){throw 'Expected one refused healthy case per lane'}
        $taskReport=$taskReports[0]
        foreach($taskName in 'returned','guard_clock_added','deadline_changed','server','product'){
            if($taskReport.$taskName -isnot [bool] -or $taskReport.$taskName){throw 'Original scope/result drift'}
        }
        if($taskReport.case -cne 'healthy' -or $null -ne $taskReport.kernel){throw 'Unexpected original refusal'}
        if($taskRaw.Contains('HC074-HELD-LIVE case=') -or $taskRaw.Contains('HC074-HELD-LIVE-COST') -or -not $taskRaw.Contains('consuming_held_live.rs:243:65:') -or -not $taskRaw.Contains('actual held case refused: healthy: exit status: 101')){throw 'Wrong failure frontier or selection'}
        if(([regex]::Matches($taskRaw,'test result: FAILED\. 0 passed; 1 failed; 0 ignored;')).Count -ne 2){throw 'Parent/child refusal missing'}
        if((Get-Content -Raw "$PSScriptRoot/remote/$taskLane-matrix.exit").Trim() -cne '101' -or (Get-Content -Raw "$PSScriptRoot/remote/$taskLane-shape.exit").Trim() -cne '1'){throw 'Original exit mismatch'}
        $taskRefusals=@($taskReport.age.sites|ForEach-Object{$_.first_refusal}|Where-Object{$null -ne $_})
        if($taskRefusals.Count -ne 1){throw 'Unexpected refusal site count'}
        $taskRefusal=$taskRefusals[0]
        if($taskRefusal.reason -cne 'expired' -or $taskRefusal.age_ns -le 10000000000 -or $taskRefusal.now_ns-$taskRefusal.issued_ns -ne $taskRefusal.age_ns -or $taskRefusal.line -ne 109 -or $taskRefusal.file -cne 'tools/long-run-supervisor-074/src/diagnostic_collector_causal.rs'){throw 'Original ten-second expiry not established'}
        $taskCausal=$taskReport.causal.first_refusal
        if($taskCausal.age_ns -ne $taskRefusal.age_ns -or $taskCausal.issued_ns -ne $taskRefusal.issued_ns -or $taskCausal.now_ns -ne $taskRefusal.now_ns -or $taskCausal.diagnostic_reason -cne 'expired' -or $taskCausal.issued_boot -cne 'e81671d6-bf23-4ebf-b8fe-5dad1f45866f' -or $taskCausal.now_boot -cne $taskCausal.issued_boot){throw 'Causal trace mismatch'}
        $taskCustody=$taskReport.age.sites[0]
        if($null -ne $taskCustody.first_refusal -or $taskCustody.first.reason -cne 'within-window' -or $taskCustody.first.age_ns -le 0 -or $taskCustody.first.age_ns -ge 10000000000){throw 'Custody observation mismatch'}
        $taskRows+=,[ordered]@{lane=$taskLane;exit=101;attempted_case='healthy';completed_cases=0;frontier='owned-root-preparation';first_refusal_file=$taskRefusal.file;first_refusal_line=$taskRefusal.line;issued_ns=$taskRefusal.issued_ns;now_ns=$taskRefusal.now_ns;age_ns=$taskRefusal.age_ns;reason=$taskRefusal.reason;first_custody_age_ns=$taskCustody.first.age_ns}
    }
    [ordered]@{source='1ecaa895fdf84114f441f95f04c6411e9389096a';image_source='b5f1b8fce6b3046239087873cb85ff1bfc557f0f';preregistration='fccac032';protected_staging='72cf0e3c';runner_sha256='5d52bde41aedce9ac1a7d5b9943d32fd3145bee8f789bfb20419cf17c8a10534';qualification_manifest_sha256='11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc';lanes=$taskRows;server=$true;product=$false;native_matrix_passed=$false;placement_reached=$false;consuming_join_reached=$false;original_send=$false;joined_sigkill_lost_ack_retested=$false;all_writer_cessation_proved=$false;later_cells_retested=$false;reconnect_retested=$false;production_enrollment=$false;earlier_failures_reclassified=$false;clock_actor_established=$false;deadline_changed=$false;live_checks_changed=$false;install_changed=$false;supervisor_changed=$false;no_retry=$true;postflight_passed=$true}
}
