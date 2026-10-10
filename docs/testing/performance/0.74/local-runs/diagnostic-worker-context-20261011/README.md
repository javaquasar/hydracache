# Local observed worker context evidence

Implementation `f861e460137d6a35ba2dc519310d4796f79aefdd` compares fixed Linux
machine/boot documents and typed current-thread user/mount namespace objects
with original signed worker policy pins. The [manifest](manifest.json) and
`verify-packet.py` bind exact raw log bytes, test inventory and closed boundaries.
Baseline `2a27e34ebf8c13ede7b8a2cdac146edfc4fbedee` precedes design commit
`68954c01287cd6b1c7e810b7b8849945943a9ba6` and missing-API E0432 refusal.

Windows baseline/final each pass 43; Linux final passes 295 with one existing
system-bus case ignored. Five integration and seven private unit cases pass
three additional repetitions. Seed `0x7542026` checks 64 valid re-signed foreign
namespace pins per run. Policy verification accepts their well-formed bytes
under synthetic new pins, but actual Linux context inspection refuses each.
Root contracts/evidence/governance pass 130; the new root guard also passes on
Linux, including actual compiled document-budget constants.

Run local packet verification from the repository root:

```powershell
python docs/testing/performance/0.74/local-runs/diagnostic-worker-context-20261011/verify-packet.py
```

Commands use Rust 1.94.0, locked inputs, two jobs, no debug info or incremental
builds. Linux runs as unprivileged `artur` in WSL; Windows excludes Linux-only
tests. Real local files are inspected without writing them; faults use private
owned temporary files or substituted guard fields. No namespaces, accounts,
services or server are changed. Synthetic pins are not real issuer enrollment
or initial-host authority. Existing receipts and production APIs remain intact.

Package check/strict lint, scoped format, docs/local performance contract, links
and book pass. Require-ship remains expected native exit 1. No full workspace
milestone gate, benchmark, host rehearsal, product workload or qualification ran.
Account/credential composition, durable epoch/refusal, all-thread authority and
authenticated start remain separate; frozen qualification bytes retain their pin.
