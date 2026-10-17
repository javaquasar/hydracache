$ErrorActionPreference = 'Stop'
$repo = (& git -C $PSScriptRoot rev-parse --show-toplevel).Trim()
$m = Get-Content -LiteralPath (Join-Path $PSScriptRoot 'manifest.json') -Raw | ConvertFrom-Json
$index = Join-Path ([IO.Path]::GetTempPath()) ('hc074-material-index-' + [Guid]::NewGuid().ToString('N'))
$savedIndex = $env:GIT_INDEX_FILE
$original = $null
try {
    $env:GIT_INDEX_FILE = $index
    foreach ($snapshot in @('candidate.patch','attribution.patch','candidate-v2.patch')) {
        & git -C $repo read-tree $m.preregister_commit
        if ($LASTEXITCODE -ne 0) { throw 'Frozen source index failed' }
        & git -C $repo apply --cached --whitespace=nowarn -- (Join-Path $PSScriptRoot $snapshot)
        if ($LASTEXITCODE -ne 0) { throw "Source capsule refused: $snapshot" }
        $actual = @(& git -C $repo diff --cached --name-only $m.preregister_commit)
        if ($LASTEXITCODE -ne 0 -or
            (($actual | Sort-Object) -join '|') -ne (($m.source_blobs.path | Sort-Object) -join '|')) { throw 'Capsule path inventory differs' }
        $runtime = $null
        foreach ($s in $m.source_blobs) {
            $line = (& git -C $repo ls-files --stage -- $s.path)
            if ($LASTEXITCODE -ne 0 -or $line -notmatch '^100644 ([a-f0-9]{40}) 0\t') { throw 'Source capsule object missing' }
            $hash = $Matches[1]
            if ($s.path -ceq 'tools/long-run-supervisor-074/src/diagnostic_original_start_material.rs') { $runtime = $hash }
            if ($snapshot -eq 'candidate-v2.patch' -and $hash -ne $s.git_blob) { throw "Active capsule differs: $($s.path)" }
        }
        if ($snapshot -eq 'candidate.patch') { $original = $runtime }
        if ($snapshot -eq 'attribution.patch' -and $runtime -ne $original) { throw 'Attribution changed candidate runtime' }
        if ($snapshot -eq 'candidate-v2.patch' -and ($null -eq $runtime -or $runtime -eq $original)) { throw 'Successor runtime identity missing or unchanged' }
        Write-Output ("Source capsule exact: $snapshot paths=10 material_runtime=$runtime")
    }
} finally {
    $env:GIT_INDEX_FILE = $savedIndex
    if (Test-Path -LiteralPath $index) { Remove-Item -LiteralPath $index -Force -ErrorAction Stop }
}
