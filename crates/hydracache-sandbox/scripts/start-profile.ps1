param(
    [ValidateSet("memory", "sqlite-memory", "sqlite-file", "postgres-compose", "postgres-docker")]
    [string] $Profile = "memory",
    [string] $Bind = "127.0.0.1:3000",
    [string] $SqlitePath = "target/hydracache-sandbox.sqlite",
    [string] $DatabaseUrl = "postgres://hydracache:hydracache@127.0.0.1:54329/hydracache"
)

$ErrorActionPreference = "Stop"

cargo run -p hydracache-sandbox -- `
    --profile $Profile `
    --bind $Bind `
    --sqlite-path $SqlitePath `
    --database-url $DatabaseUrl
