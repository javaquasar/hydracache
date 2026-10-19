$ErrorActionPreference = 'Stop'
$taskRows = @()
foreach ($taskCohort in @('gnu194-confirmed','gnu188-confirmed')) {
    foreach ($taskRepeat in 1..3) {
        $taskLog = Get-Content -Raw -LiteralPath (Join-Path $PSScriptRoot "$taskCohort/preparation-$taskRepeat.log")
        foreach ($taskMatch in [regex]::Matches($taskLog,'HC074-PREPARATION (\{[^\r\n]+\})')) {
            $taskRow = $taskMatch.Groups[1].Value | ConvertFrom-Json
            $taskOwn = ($taskRow.kernel | Measure-Object -Property outside_children_ns -Sum).Sum
            $taskHot = $taskRow.kernel | Sort-Object outside_children_ns -Descending | Select-Object -First 1
            $taskFailure = $taskRow.clock.first_refusal
            $taskRows += [ordered]@{
                cohort=$taskCohort;repeat=$taskRepeat;phase=$taskRow.phase
                wall_ms=$taskRow.wall_ns/1e6;auth_calls=$taskRow.auth_calls
                auth_ms=$taskRow.auth_ns/1e6;signature_ms=$taskRow.signature_ns/1e6
                kernel_nested_coverage_ms=$taskOwn/1e6;hottest_kernel_index=$taskHot.index
                hottest_kernel_outside_children_ms=$taskHot.outside_children_ns/1e6
                observed_points=$taskRow.clock.observations
                first_age_ns=$taskRow.clock.first.age_ns;last_age_ns=$taskRow.clock.last.age_ns
                first_refusal=$taskFailure;wall_not_cpu=$true;inclusive_spans_overlap=$true
            }
        }
    }
}
$taskRows | ConvertTo-Json -Depth 8
