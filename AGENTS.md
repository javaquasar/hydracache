# HydraCache agent instructions

Follow the repository bootstrap in [CLAUDE.md](CLAUDE.md).

## Temporary shell scripts

- Keep the checkout root free of new temporary `.sh` files, including files
  created from WSL, PowerShell, and scheduled tasks.
- Run one-off commands directly. Reuse a parameterized script for repeated
  operations. Put new maintained scripts under `scripts/`.
- If a temporary shell script is necessary, use
  `tools/Save-TemporaryShellScript.ps1` with PowerShell 7. It stores scripts as
  `.tmp/generated-shell/<task>/<operation>.sh` and limits them to three per
  task and twenty per checkout. Use the same task and operation names on
  retries. Pass `-Replace` to change an existing script intentionally.
- After the task, remove its temporary scripts and empty task directory. Keep
  scripts needed to reproduce a run with that run's evidence.

Run the offline limit checks with
`pwsh -NoProfile -File scripts/test-temporary-shell-script-limit.ps1`.
