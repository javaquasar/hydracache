function Read-AllocationRows([string]$Raw) {
    $rows=@()
    $prefix='test fixed_boot_reader_allocation_pairs ... '
    foreach ($line in ($Raw.Replace("`r",'') -split "`n")) {
        if (!$line.Contains('HC074-BOOT-ALLOCATION')) {continue}
        $value=if ($line.StartsWith($prefix,[StringComparison]::Ordinal)) {$line.Substring($prefix.Length)} else {$line}
        if ($value -cnotmatch '^HC074-BOOT-ALLOCATION scope=([0-4]) reads=32 baseline_allocations=32 baseline_requested=2097184 baseline_deallocations=32 candidate_allocations=0 candidate_requested=0 candidate_deallocations=0 whole_worker=false$') {throw 'allocation row contract'}
        $rows+=@{scope=[int]$Matches[1];reads=32;baseline_allocations=32;baseline_requested=2097184;baseline_deallocations=32;candidate_allocations=0;candidate_requested=0;candidate_deallocations=0;whole_worker=$false}
    }
    if ($rows.Count -ne 5 -or (($rows.scope -join ',') -cne '0,1,2,3,4')) {throw 'allocation scope inventory'}
    if ($Raw -cnotmatch 'test result: ok\. 4 passed; 0 failed; 0 ignored;') {throw 'allocation suite not green'}
    return ,$rows
}
