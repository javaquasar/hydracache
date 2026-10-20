$ErrorActionPreference='Stop'
function PacketJson([string]$Name){Get-Content -Raw -LiteralPath "$PSScriptRoot/$Name"|ConvertFrom-Json}
function Markers([string]$Name,[string]$Prefix){
    @(foreach($taskLine in Get-Content -LiteralPath "$PSScriptRoot/$Name"){
        $taskAt=$taskLine.IndexOf($Prefix,[StringComparison]::Ordinal)
        if($taskAt -ge 0){$taskLine.Substring($taskAt+$Prefix.Length)|ConvertFrom-Json}
    })
}
function PreparationFacts {
    $taskAllocationRounds=0
    $taskBefore=@();$taskAfter=@();$taskMatrices=@()
    foreach($taskLane in '194','188'){
        foreach($taskArm in 'red','green'){
            $taskExit=PacketJson "prefix-$taskArm-$taskLane-exit.json"
            $taskExpectedExit=if($taskArm -ceq 'red'){101}else{0}
            if($taskExit.exit -ne $taskExpectedExit){throw 'Leaf outcome differs'}
            $taskRows=@(Markers "prefix-$taskArm-$taskLane.log" 'HC074-PROJECTION-PREFIX-ALLOC ')
            if($taskRows.Count -ne 3){throw 'Missing paired allocation samples'}
            foreach($taskSample in 0..2){
                $taskRow=$taskRows[$taskSample]
                if($taskRow.sample -ne $taskSample -or $taskRow.iterations -ne 4096 -or $taskRow.candidate_first -cne ($taskSample%2 -eq 1) -or $taskRow.product -cne $false -or $taskRow.rust_requests_only -cne $true){throw 'Leaf workload changed'}
                $taskCandidate=if($taskArm -ceq 'red'){@(40960,491520)}else{@(20480,327680)}
                if($taskRow.reference.allocations -ne 40960 -or $taskRow.reference.requested_bytes -ne 491520 -or $taskRow.reference.reallocations -ne 0 -or $taskRow.candidate.allocations -ne $taskCandidate[0] -or $taskRow.candidate.requested_bytes -ne $taskCandidate[1] -or $taskRow.candidate.reallocations -ne 0){throw 'Allocation effect not reproduced'}
                if($taskArm -ceq 'green'){$taskAllocationRounds++}
            }
            $taskTiming=@(Markers "prefix-$taskArm-$taskLane.log" 'HC074-PROJECTION-PREFIX-TIME ')
            if($taskTiming.Count -ne 3){throw 'Missing separate timing samples'}
            foreach($taskSample in 0..2){
                if($taskTiming[$taskSample].sample -ne $taskSample -or $taskTiming[$taskSample].iterations -ne 32768 -or $taskTiming[$taskSample].allocation_counter_active -cne $false -or $taskTiming[$taskSample].product -cne $false){throw 'Timing is not separate or workload changed'}
            }
            $taskLog=Get-Content -Raw "$PSScriptRoot/prefix-$taskArm-$taskLane.log"
            if($taskArm -ceq 'red' -and -not $taskLog.Contains('fixed prefixes must remove repeated allocation requests')){throw 'Wrong initial failing assertion'}
            if(-not $taskLog.Contains('HC074-PROJECTION-DIFFERENTIAL seed=0x07401d5e mutations=1024 original_refusals=true product=false')){throw 'Differential inputs not checked'}
        }
        foreach($taskCohort in 'instrumented','candidate'){
            $taskDir=if($taskCohort -ceq 'candidate'){'reviewed/'}else{''}
            $taskExits=@();$taskPhases=0
            foreach($taskSample in 0..2){
                $taskName="$taskDir$taskCohort-$taskLane-profile-$taskSample"
                $taskExit=PacketJson "$taskName-exit.json"
                if($taskExit.exit -notin @(0,101)){throw 'Unexpected phase attempt outcome'}
                $taskExits+=$taskExit.exit
                $taskRows=@(Markers "$taskName.log" 'HC074-HELD-PHASE ')
                foreach($taskRow in $taskRows){
                    foreach($taskFlag in 'guard_clock_added','deadline_changed','includes_child_cpu','healthy_gate','server','product'){
                        if($taskRow.$taskFlag -isnot [bool] -or $taskRow.$taskFlag -cne $false){throw 'Phase scope upgrade'}
                    }
                    if(@($taskRow.kernel).Count -ne 10 -or @($taskRow.live).Count -ne 6){throw 'Missing original span families'}
                    if($taskRow.phase -ceq 'outputs' -and $taskRow.returned -ceq $true -and $taskRow.kernel[3].calls -ne 39744){throw 'Completed outputs projection count changed'}
                }
                $taskPhases+=$taskRows.Count
                $taskOriginal=@(Markers "$taskName.log" 'HC074-HELD-LIVE-ORIGINAL ')
                if($taskOriginal.Count -ne 1 -or $taskOriginal[0].case -cne 'healthy' -or $taskOriginal[0].deadline_changed -cne $false -or $taskOriginal[0].guard_clock_added -cne $false){throw 'Original outcome report missing'}
                if($taskOriginal[0].returned -cne ($taskExit.exit -eq 0)){throw 'Original result does not match exit'}
            }
            $taskValue=[ordered]@{lane=$taskLane;exits=$taskExits;phase_reports=$taskPhases}
            if($taskCohort -ceq 'instrumented'){$taskBefore+=$taskValue}else{$taskAfter+=$taskValue}
        }
        $taskMatrix=PacketJson "reviewed/candidate-$taskLane-matrix-exit.json"
        $taskRaw=Get-Content -Raw "$PSScriptRoot/reviewed/candidate-$taskLane-matrix.log"
        if($taskRaw.Contains('HC074-HELD-PHASE ') -or $taskRaw.Contains('HC074-HELD-LIVE-COST ')){throw 'Functional matrix was profiled'}
        $taskCompleted=([regex]::Matches($taskRaw,'HC074-HELD-LIVE case=')).Count
        $taskPassed=$taskMatrix.exit -eq 0
        $taskReports=@(Markers "reviewed/candidate-$taskLane-matrix.log" 'HC074-HELD-LIVE-ORIGINAL ')
        $taskFailedCase=$null;$taskRefusalSite=$null
        if($taskPassed){
            if($taskCompleted -ne 5 -or ([regex]::Matches($taskRaw,'test result: ok\. 1 passed; 0 failed; 0 ignored;')).Count -ne 6){throw 'Incomplete matrix cannot pass'}
            foreach($taskCase in 'healthy','revalidate','role-before','role-after','trust-after'){
                if(([regex]::Matches($taskRaw,[regex]::Escape("HC074-HELD-LIVE case=$taskCase seed=0x074c051e actual_fixed=true actual_consuming=true"))).Count -ne 1){throw 'Matrix case missing or duplicated'}
            }
        }else{
            if($taskMatrix.exit -ne 101 -or $taskReports.Count -ne $taskCompleted+1){throw 'Unexpected or incomplete matrix failure'}
            $taskFailed=$taskReports[-1]
            if($taskFailed.returned -cne $false -or $taskFailed.kernel.site -cne 'clock-order' -or $taskFailed.kernel.clock[4] -ne 1 -or $taskFailed.kernel.clock[1] -lt $taskFailed.kernel.clock[0] -or $taskFailed.kernel.clock[3] -ge $taskFailed.kernel.clock[2]){throw 'Retained original clock-order refusal missing'}
            $taskFailedCase=$taskFailed.case;$taskRefusalSite=$taskFailed.kernel.site
        }
        $taskMatrices+=[ordered]@{lane=$taskLane;exit=$taskMatrix.exit;completed_cases=$taskCompleted;passed=$taskPassed;failed_case=$taskFailedCase;original_refusal_site=$taskRefusalSite}
    }
    [ordered]@{
        schema='held-preparation-cost-local-v1'
        reference_source='b41a7fb652f0687fbe5bad3222b5fa82f5e09443'
        instrumentation_source='754269aa64a4bd2d5a6da93629976d313f7e7c1a'
        runtime_candidate_source='1a3285938208760e64b6683e54da808bd338c2ad'
        reviewed_run_source='601f39ff72b20014cc773d012ca69ea978aea7a7'
        allocation_samples=$taskAllocationRounds
        reference_allocations_per_decode=10
        candidate_allocations_per_decode=5
        reference_requested_bytes_per_decode=120
        candidate_requested_bytes_per_decode=80
        allocation_candidate_accepted=($taskAllocationRounds -eq 6)
        before=$taskBefore
        after=$taskAfter
        matrices=$taskMatrices
        full_local_matrix_passed=(@($taskMatrices|Where-Object{$_.passed -cne $true}).Count -eq 0)
        reconstructed_old_files=462
        live_checks_changed=$false
        original_origins_changed=$false
        deadline_changed=$false
        qualification_changed=$false
        native_server_retested=$false
        native_execution_gate_passed=$false
        original_send=$false
        full_joined_sigkill=$false
        lost_ack_checked=$false
        real_role_image_cost=$false
        production_enrollment=$false
        end_to_end_latency_claim=$false
        numerical_product_claim=$false
        clock_actor_established=$false
        server=$false
        product=$false
    }
}
