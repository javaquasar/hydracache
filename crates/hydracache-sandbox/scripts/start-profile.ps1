param(
    [ValidateSet("memory", "sqlite-memory", "sqlite-file", "postgres-docker")]
    [string] $Profile = "memory",
    [string] $Bind = "127.0.0.1:3000",
    [string] $SqlitePath = "target/hydracache-sandbox.sqlite"
)

$ErrorActionPreference = "Stop"

cargo run -p hydracache-sandbox -- `
    --profile $Profile `
    --bind $Bind `
    --sqlite-path $SqlitePath
