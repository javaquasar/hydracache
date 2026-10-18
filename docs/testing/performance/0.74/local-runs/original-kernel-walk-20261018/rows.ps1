# Bounded numeric parser and independent call-tree accounting, no test execution.
function Require-Kernel([bool]$Test,[string]$Reason) {if (!$Test) {throw $Reason}}
function Read-KernelRows([string]$Raw) {
    $rows=@();$controls=@();$compiler=''
    $edgeSpecs=@(@(0,1,128),@(0,8,64),@(0,7,64),@(1,4,256),@(1,2,256),@(1,3,256),@(1,9,256),@(4,5,1024),@(4,6,2048),@(2,9,256),@(5,9,1024),@(7,4,128),@(8,4,128))
    foreach ($line in ($Raw -split "\r?\n")) {
        $prefix='test diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::tests::fence_preflight_tests::attempt_tests::collector_tests::observed_reservation_tests::collector_reserved_tests::progress_tests::start_consent_tests::validation_frame_tests::starting_tests::sink_tests::start_material_tests::pipe_transport_tests::safety_alarm_tests::leaf_cost_tests::component_cost_tests::live_walk_tests::kernel_walk_tests::original_kernel_context_walk_attribution ... '
        if ($line.StartsWith($prefix,[StringComparison]::Ordinal)) {$line=$line.Substring($prefix.Length)}
        if ($line -cmatch '^HC074-KERNEL-HOST compiler=(194|188) begin=true$') {$compiler=$Matches[1]}
        if ($line -cmatch '^HC074-KERNEL-(WORKER|NODE) sample=([0-2]) index=([0-9]) stats=Stats \{ calls: ([0-9]+), inclusive_ns: ([0-9]+), outside_children_ns: ([0-9]+) \}$') {
            $rows+=@{compiler=$compiler;type=$Matches[1];sample=[int]$Matches[2];index=[int]$Matches[3];parent=-1;child=-1;calls=[long]$Matches[4];inclusive_ns=[long]$Matches[5];outside_children_ns=[long]$Matches[6]}
        }
        if ($line -cmatch '^HC074-KERNEL-EDGE sample=([0-2]) parent=([0-9]) child=([0-9]) calls=([0-9]+) inclusive_ns=([0-9]+)$') {
            $rows+=@{compiler=$compiler;type='EDGE';sample=[int]$Matches[1];index=-1;parent=[int]$Matches[2];child=[int]$Matches[3];calls=[long]$Matches[4];inclusive_ns=[long]$Matches[5];outside_children_ns=0}
        }
        if ($line -cmatch '^HC074-KERNEL-CONTROL sample=([0-2]) iterations=32 roots=64 original=true unchanged=true reservation=false schedule_changed=false admission=false$') {$controls+="$compiler/$($Matches[1])"}
    }
    Require-Kernel ($rows.Count -eq 174 -and $controls.Count -eq 6) 'row/control inventory'
    foreach ($c in @('194','188')) {
        Require-Kernel ($Raw.Contains("HC074-KERNEL-HOST compiler=$c completed=true admission=false")) 'compiler not complete'
        foreach ($sample in 0..2) {
            Require-Kernel (@($controls | Where-Object {$_ -ceq "$c/$sample"}).Count -eq 1) 'control membership'
            $batch=@($rows | Where-Object {$_.compiler -ceq $c -and $_.sample -eq $sample})
            $workers=@($batch | Where-Object {$_.type -ceq 'WORKER'})
            $nodes=@($batch | Where-Object {$_.type -ceq 'NODE'})
            $edges=@($batch | Where-Object {$_.type -ceq 'EDGE'})
            Require-Kernel ($batch.Count -eq 29 -and $workers.Count -eq 6 -and $nodes.Count -eq 10 -and $edges.Count -eq 13) 'batch inventory'
            $workerCounts=@(32,64,128,128,64,32)
            $nodeCounts=@(64,128,256,256,512,1024,2048,64,64,1536)
            for ($i=0;$i -lt 6;$i++) {Require-Kernel ($workers[$i].index -eq $i -and $workers[$i].calls -eq $workerCounts[$i]) 'worker counters/order'}
            for ($i=0;$i -lt 10;$i++) {
                Require-Kernel ($nodes[$i].index -eq $i -and $nodes[$i].calls -eq $nodeCounts[$i] -and $nodes[$i].outside_children_ns -le $nodes[$i].inclusive_ns) 'node counters/order'
                $incoming=@($edges | Where-Object {$_.child -eq $i})
                $children=@($edges | Where-Object {$_.parent -eq $i})
                $incomingCalls=[long]($incoming | Measure-Object calls -Sum).Sum
                $incomingTime=[long]($incoming | Measure-Object inclusive_ns -Sum).Sum
                $childrenTime=[long]($children | Measure-Object inclusive_ns -Sum).Sum
                $root=if($i -eq 0){64}else{0}
                Require-Kernel ($nodes[$i].calls -eq ($root+$incomingCalls)) 'node incoming/root count'
                if ($i -ne 0) {Require-Kernel ($nodes[$i].inclusive_ns -eq $incomingTime) 'node incoming time'}
                Require-Kernel (($nodes[$i].inclusive_ns-$nodes[$i].outside_children_ns) -eq $childrenTime) 'parent/direct-child time'
            }
            for ($i=0;$i -lt 13;$i++) {
                Require-Kernel ($edges[$i].parent -eq $edgeSpecs[$i][0] -and $edges[$i].child -eq $edgeSpecs[$i][1] -and $edges[$i].calls -eq $edgeSpecs[$i][2]) 'edge membership/order'
            }
            Require-Kernel (($workers[0].inclusive_ns-$workers[0].outside_children_ns) -eq ($workers[1].inclusive_ns+$workers[4].inclusive_ns+$workers[5].inclusive_ns)) 'worker nesting'
            Require-Kernel (($workers[1].inclusive_ns-$workers[1].outside_children_ns) -eq ($workers[2].inclusive_ns+$workers[3].inclusive_ns)) 'policy nesting'
            Require-Kernel ($nodes[0].inclusive_ns -le $workers[4].inclusive_ns) 'enclosing kernel span'
            Require-Kernel (($nodes | Measure-Object outside_children_ns -Sum).Sum -eq $nodes[0].inclusive_ns) 'disjoint node partition'
            foreach ($i in @(2,3,4,5)) {Require-Kernel ($workers[$i].inclusive_ns -eq $workers[$i].outside_children_ns) 'worker leaf nesting'}
        }
    }
    Require-Kernel ($Raw.Contains('HC074-KERNEL-HOST completed=true supervisor_unchanged=true no_fixture_executable=true admission=false')) 'host not complete'
    Require-Kernel (($Raw -split 'test result: ok\. 1 passed; 0 failed; 0 ignored; 0 measured;').Count -eq 3) 'host test outcomes'
    return ,$rows
}
