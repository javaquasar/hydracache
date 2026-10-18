# Barred collector notice parser units

Only the parser-only part of the two-child experiment is accepted. Three new
fast Linux tests exercise initial B, later/duplicate/empty/unknown/oversized
notices, E handling, peer disconnect and alarm/terminal readiness precedence.
Synthetic event FDs and pid=0 make cancellation inert; these are parser units,
not process-identity, original-alarm or original-owner-death proofs.

The entire ordinary kernel and original join/test APIs remain byte exact to
1c56d4a3 after removing one cfg(test) module registration. Two static contracts
freeze that boundary and check the notice corpus. No fixture authority,
production API, wire vocabulary, clock origin, deadline or validation frame is
changed. The unaccepted owner-loss/lost-B fixture is archived separately in
../original-collector-loss-20261018 and withdrawn from working source.

Current/MSRV release-libtest images run eight descriptor/notice units each.
Local gates cover all-target checks and strict Clippy on GNU Rust 1.94/1.88 and
Windows, 221 static contracts, formatting, registry/docs and the non-promotable
performance contract. Full release verification, installation, product load and
qualification are not part of this step. The known MSRV healthy-window refusal
remains unresolved; passing parser units does not admit the stopped process
matrix, original send/start, active writers or completion.

The source commit, four-file patch and 321 package source/manifest/lock inputs are
bound in manifest.json. Run ./verify.ps1 -SelfTest in PowerShell for closed-file,
hash, Git-source, compiler-image and outcome validation. No performance win is
claimed and qualification inputs are unchanged.
