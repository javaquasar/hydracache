# Original intent and fenced Reserved state composition

This local W11 prerequisite joins the already original authority/signed-intent
guard with the already original Reserved-state/common-fence guard. Baseline is
`0399d63bc1fb2020a26d522cc007d465ada59018`. It changes no host ownership,
installer, publisher, state format, CLI or start route.

The fixed binder accepts only FixedAuthorityIntentRead; the fixture binder
accepts only FixtureAuthorityIntentRead. Each exclusively borrows the matching
intent and FencedReservedStateRead, never rebuilding worker authority, refreshing
state or acquiring another lock. The fixture wrapper cannot promote into the
fixed API. Neither wrapper is cloneable, Send/Sync or an execution capability.

Before composition I/O, require the complete asserted Reserved snapshot to
match both the original signed-intent pins and original fenced reader's
expected state. A wrong unbound join assertion refuses without revoking healthy
inputs. Already refused inputs instead latch both original chains in memory and
return without further filesystem, kernel or journal I/O.

Revalidation is fence/intent/fence/intent/fence. Every intent step retains its
existing authority/intent/authority and journal/signed-runtime/journal checks.
Every fence step retains its original file/state/fence brackets. No cached
success replaces a later observation. This sequential bracket is not an atomic
whole-host snapshot or continuous confinement.

A bound failure latches both original chains, including constructor inspection
failure and wrapper drop. Intent failure preserves its existing typed error
and original authority refusal behavior without duplicate publication. State
or fence failure also attempts Policy refusal only through the independently
exact-bound original authority journal. Unsafe or advanced authority is not
repaired, adopted or appended. Failed refusal acknowledgement reports the
first fence error plus typed publication uncertainty. Subsequent calls refuse
before I/O, even after restoring files. Successful wrapper drop does not unlock
the caller's still-original fenced guard or mutate state; that guard owns the
existing flock.

Use only local owned kernel helpers, synthetic signatures, nonroot authority
journals and privately modeled fixed state/fence ancestry. Test healthy repeated
observation/drop, full-state mismatches before I/O, all bracket first-error
positions, bound intent drift, original state/name/context drift, late changes,
independent journal safety/epoch failures, refusal through drop, already-refused
no-I/O, fixed/fixture non-promotion and independent concurrency. Retain baseline,
missing API red, final repeats, regressions, lint/MSRV/format/docs and genuine
qualification/ship negatives. No timing or product claim follows.

Coordinated common-binary deployment and quiescent trusted ownership enrollment
still precede publication on a host. Trusted reservation/consumption and fixed
intent publication, authenticated peer/clock, output/start/watchdog/owned cleanup
remain separate packages. All frozen qualification inputs and the inherited
plan-digest refusal stay unchanged.
