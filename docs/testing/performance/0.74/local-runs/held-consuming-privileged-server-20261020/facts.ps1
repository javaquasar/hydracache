$ErrorActionPreference='Stop'
function ControlFacts {
    $taskRows=@()
    foreach($taskLane in '194','188'){
        $taskRaw=Get-Content -Raw -LiteralPath "$PSScriptRoot/remote/$taskLane-matrix.log"
        $taskReports=@(Get-Content "$PSScriptRoot/remote/$taskLane-matrix.log"|Where-Object{$_.StartsWith('HC074-HELD-LIVE-ORIGINAL ',[StringComparison]::Ordinal)}|ForEach-Object{$_.Substring(25)|ConvertFrom-Json})
        if($taskReports.Count -ne 1){throw 'Expected one retained refused original report'}
        $taskReport=$taskReports[0]
        foreach($taskField in 'returned','guard_clock_added','deadline_changed','server','product'){
            if($taskReport.$taskField -isnot [bool] -or $taskReport.$taskField -cne $false){throw 'Report scope or result changed'}
        }
        if($taskReport.case -cne 'healthy' -or $null -ne $taskReport.kernel){throw 'Unexpected refusal case or kernel report'}
        if($taskReport.causal.first.issued_boot -cne 'e81671d6-bf23-4ebf-b8fe-5dad1f45866f' -or $taskReport.causal.first.now_boot -cne $taskReport.causal.first.issued_boot){throw 'Wrong boot context'}
        if($taskRaw.Contains('HC074-HELD-LIVE case=') -or $taskRaw.Contains('HC074-HELD-LIVE-COST') -or -not $taskRaw.Contains('consuming_held_live.rs:250:77:') -or -not $taskRaw.Contains('actual held case refused: healthy: exit status: 101')){throw 'Selection or failure frontier differs'}
        if(([regex]::Matches($taskRaw,'test result: FAILED\. 0 passed; 1 failed; 0 ignored;')).Count -ne 2){throw 'Parent and child result missing'}
        if((Get-Content -Raw "$PSScriptRoot/remote/$taskLane-matrix.exit").Trim() -cne '101' -or (Get-Content -Raw "$PSScriptRoot/remote/$taskLane-shape.exit").Trim() -cne '1'){throw 'Exit changed'}
        $taskSite=if($taskLane -ceq '194'){$taskReport.age.sites[1]}else{$taskReport.age.sites[0]}
        $taskRefusal=$taskSite.first_refusal
        if($taskRefusal.reason -cne 'expired' -or $taskRefusal.age_ns -le 10000000000 -or $taskRefusal.now_ns-$taskRefusal.issued_ns -ne $taskRefusal.age_ns){throw 'Original expiry not established'}
        $taskRows += [ordered]@{lane=$taskLane;exit=101;completed_cases=0;attempted_case='healthy';phase='signed-placement-bind';first_refusal_file=$taskRefusal.file;first_refusal_line=$taskRefusal.line;issued_ns=$taskRefusal.issued_ns;now_ns=$taskRefusal.now_ns;age_ns=$taskRefusal.age_ns;reason='expired';first_custody_age_ns=$taskReport.age.sites[0].first.age_ns}
    }
    return [ordered]@{
        source='1eac3bb37814d5aaf44f84370606218719c01c2f'
        preregistration='09d1c582cdb7f87d1ba309c5b11e4ca16c52ca7f'
        staging_recipe_commit='258a43be49aeb6e1b2ae441a9d604347a4f8f6f0'
        runner_sha256='f18122dfa990b0044838db11c6d9291dd54e37fbe0492f675d04cfd4ce3bf109'
        qualification_manifest_sha256='11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc'
        lanes=$taskRows
        server=$true
        product=$false
        original_send=$false
        native_matrix_passed=$false
        full_joined_sigkill=$false
        lost_ack_checked=$false
        real_role_image_cost=$false
        production_enrollment=$false
        root_guardians_terminal_claim=$false
        clock_actor_established=$false
        earlier_wsl_cohorts_accepted=$false
        deadline_changed=$false
        live_checks_changed=$false
        install_changed=$false
        supervisor_changed=$false
        no_retry=$true
        postflight_passed=$true
    }
}
