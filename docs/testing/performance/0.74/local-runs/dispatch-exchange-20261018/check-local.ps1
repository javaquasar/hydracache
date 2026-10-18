$ErrorActionPreference='Stop'
wsl -d Ubuntu -- /home/artur/.cargo/bin/cargo +1.94.0 fmt --all -- --check
if ($LASTEXITCODE -ne 0) {throw 'format'}
cargo xtask gated-test-check
if ($LASTEXITCODE -ne 0) {throw 'registry'}
cargo xtask doc-check
if ($LASTEXITCODE -ne 0) {throw 'docs'}
cargo xtask performance-contract-check --release 0.74
if ($LASTEXITCODE -ne 0) {throw 'local contract'}
'HC074-DISPATCH-LOCAL completed=true admission=false'
