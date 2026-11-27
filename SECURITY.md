# Security policy

## Supported versions

Security fixes target the current stable release. The 0.74 maintenance line
includes the 0.74.0 server binary and the 0.74.1 crates on crates.io. For
earlier releases, upgrade to the current stable release before relying on a fix.

## Reporting a vulnerability

Use [GitHub private vulnerability reporting](https://github.com/javaquasar/hydracache/security/advisories/new).
Include the affected package and version, impact and the information needed
to verify the report privately. We aim to acknowledge reports within seven
calendar days.

After confirming a report, maintainers coordinate a fixed release and a GitHub
Security Advisory, request a CVE where appropriate, submit a RustSec advisory
and yank affected crates.io versions as appropriate. The
[advisory procedure](docs/SECURITY_ADVISORIES.md) describes the sequence.

## Forked dependencies

HydraCache depends on maintained `hydra-*` forks of raft-rs, sled and SQLx.
Report an issue in a fork through that fork's private reporting, or here if
you are unsure which project is affected:

| Fork repository | crates.io packages |
| --- | --- |
| [javaquasar/raft-rs](https://github.com/javaquasar/raft-rs) | `hydra-raft`, `hydra-raft-proto` |
| [javaquasar/sled](https://github.com/javaquasar/sled) | `hydra-sled` |
| [javaquasar/sqlx](https://github.com/javaquasar/sqlx) | `hydra-sqlx`, `hydra-sqlx-sqlite`, `hydra-sqlx-macros`, `hydra-sqlx-macros-core` |

The [fork decisions](docs/security/fork-decisions.md) record why each fork
exists and when it is retired.
