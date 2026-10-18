$ErrorActionPreference='Stop'
cargo check -p hydracache-long-run-supervisor-074 -p xtask --all-targets --locked
if ($LASTEXITCODE -ne 0) {throw 'Windows check'}
cargo clippy -p hydracache-long-run-supervisor-074 -p xtask --all-targets --all-features --locked -- -D warnings
if ($LASTEXITCODE -ne 0) {throw 'Windows Clippy'}
cargo test -p hydracache-long-run-supervisor-074 --lib --locked
if ($LASTEXITCODE -ne 0) {throw 'Windows controls'}
'HC074-DISPATCH-WINDOWS completed=true admission=false'
