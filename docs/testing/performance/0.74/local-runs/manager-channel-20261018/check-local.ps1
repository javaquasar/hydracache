$ErrorActionPreference='Stop'
wsl -d Ubuntu -- /home/artur/.cargo/bin/cargo +1.94.0 fmt --all -- --check
if ($LASTEXITCODE -ne 0) {throw 'format'}
cargo check -p hydracache-long-run-supervisor-074 -p xtask --all-targets --locked
if ($LASTEXITCODE -ne 0) {throw 'Windows check'}
cargo clippy -p hydracache-long-run-supervisor-074 -p xtask --all-targets --all-features --locked -- -D warnings
if ($LASTEXITCODE -ne 0) {throw 'Windows Clippy'}
cargo test -p hydracache-long-run-supervisor-074 --lib --locked
if ($LASTEXITCODE -ne 0) {throw 'Windows controls'}
cargo xtask gated-test-check
if ($LASTEXITCODE -ne 0) {throw 'registry'}
cargo xtask doc-check
if ($LASTEXITCODE -ne 0) {throw 'docs'}
cargo xtask performance-contract-check --release 0.74
if ($LASTEXITCODE -ne 0) {throw 'local contract'}
'HC074-MANAGER-CHECKS completed=true admission=false'
