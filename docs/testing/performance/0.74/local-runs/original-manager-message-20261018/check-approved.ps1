$ErrorActionPreference='Stop'
& {
    git diff --exit-code 95945625 -- tools/long-run-supervisor-074/src crates/xtask/tests/support docs/COMPAT.md docs/testing/gated-test-registry.toml
    if($LASTEXITCODE -ne 0){throw 'runtime withdrawal'}
    cargo fmt -p hydracache-long-run-supervisor-074 -p xtask -- --check
    if($LASTEXITCODE -ne 0){throw 'format'}
    cargo xtask gated-test-check
    if($LASTEXITCODE -ne 0){throw 'registry'}
    cargo xtask doc-check
    if($LASTEXITCODE -ne 0){throw 'docs'}
    cargo xtask performance-contract-check --release 0.74
    if($LASTEXITCODE -ne 0){throw 'contract'}
    git diff --check
    if($LASTEXITCODE -ne 0){throw 'diff'}
    'HC074-ADDRESSED-WITHDRAWAL runtime_matches_base=true admission=false'
} 2>&1 | Tee-Object -FilePath (Join-Path $PSScriptRoot 'approved-final.log')
