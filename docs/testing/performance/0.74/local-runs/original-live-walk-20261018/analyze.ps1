$ErrorActionPreference='Stop'
$raw=Get-Content (Join-Path $PSScriptRoot 'host.log') -Raw
$compiler=''
$rows=@()
foreach ($line in ($raw -split "\r?\n")) {
    $prefix='test diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::tests::fence_preflight_tests::attempt_tests::collector_tests::observed_reservation_tests::collector_reserved_tests::progress_tests::start_consent_tests::validation_frame_tests::starting_tests::sink_tests::start_material_tests::pipe_transport_tests::safety_alarm_tests::leaf_cost_tests::component_cost_tests::live_walk_tests::original_live_worker_walk_attribution ... '
    if ($line.StartsWith($prefix,[StringComparison]::Ordinal)) {$line=$line.Substring($prefix.Length)}
    if ($line -cmatch '^HC074-LIVE-WALK-HOST compiler=(194|188) begin=true$') {$compiler=$Matches[1]}
    if ($line -cmatch '^HC074-LIVE-WALK sample=([0-2]) kind=([a-z-]+) stats=Stats \{ calls: ([0-9]+), inclusive_ns: ([0-9]+), outside_children_ns: ([0-9]+) \}$') {
        $rows+=@{compiler=$compiler;sample=[int]$Matches[1];kind=$Matches[2];calls=[long]$Matches[3];inclusive_ns=[long]$Matches[4];outside_children_ns=[long]$Matches[5]}
    }
}
if ($rows.Count -ne 36) {throw 'row inventory'}
@{unit='nanoseconds';iterations_per_sample=32;rows=$rows;exclusive_cpu_claim=$false;critical_path_claim=$false;optimization_accepted=$false} | ConvertTo-Json -Depth 8 | Set-Content -Encoding utf8NoBOM (Join-Path $PSScriptRoot 'observations.json')
foreach ($compiler in @('194','188')) {
    foreach ($kind in @('kernel-worker','worker-policy','worker-source','active-authority','kernel-context','local-context')) {
        $selected=@($rows | Where-Object {$_.compiler -ceq $compiler -and $_.kind -ceq $kind})
        $range=$selected | ForEach-Object {$_.inclusive_ns / 32000000.0} | Measure-Object -Minimum -Maximum
        "compiler=$compiler kind=$kind per_worker_ms=$($range.Minimum.ToString('F6',[Globalization.CultureInfo]::InvariantCulture))..$($range.Maximum.ToString('F6',[Globalization.CultureInfo]::InvariantCulture))"
    }
}
'HC074-LIVE-WALK-ANALYZE rows=36 numeric_only=true admission=false'
