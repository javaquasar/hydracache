# Existing fence before initial reservation

An initial Reserved-state publisher cannot use the original Reserved reader's
fence: that reader requires an already existing state file. This local W11
prerequisite removes only that circular dependency. Baseline is
`b418c820e39f1c8e475cf6c73728409aaec314ab`. No publisher, state creation,
host ownership change, installer or execution route is added.

The new guard exclusively borrows existing FixedIntentStorageRead of the intent
origin and reuses the same trusted campaign Fence engine. Require the original
private trusted-owner campaigns directory and existing regular empty one-link
0600 lock on the same device. Do not create a lock, repair permissions, copy
state or promote the deployed worker-owned 0750 layout into root authority.

After the existing nonblocking exclusive flock is held, bracket fresh
descriptor-relative scans of the retained fixed leaf with original ancestry,
campaigns-directory, named/original lock and legacy-context checks. Every entry
refuses: known, pending, terminal, unknown, directory, FIFO, symlink or non-UTF-8.
Campaign/legacy diagnostic/IPC and parent lifecycle/controller-loss contexts
also refuse, including dangling links. Missing/unsafe original objects are
errors, never absence. The existing scanner opens an independent description
for each scan rather than sharing directory enumeration offsets.

Initial Busy is retryable contention without storage revocation or unlocking
the other owner. Once inspection under the new guard starts, any error latches
the original storage through restoration and wrapper drop. Already refused
calls return before I/O. A wrong private modeled authority origin refuses
before I/O without damaging an unrelated healthy storage origin.
Healthy drop releases only the guard's own retained flock, with no file removal
or mutation. The empty observation expires with the guard; it is neither a
durable reservation nor an execution or write capability.

There is no public path, owner, FD, state getter, cloning, Send/Sync, fixture
promotion, independent lock or execution conversion. Test only private nonroot
modeled ancestry and real temporary locks. Prove both acquisition orders with
legacy campaign APIs, all conflict kinds, unsafe/missing lock and ancestry,
named/original object drift, repeat scans, seeded late context, sticky refusal,
healthy release and independent concurrent chains. Keep baseline, absent API
red, repeated checks, regressions, lint/current/MSRV/format/docs and genuine
qualification/ship refusals. These are local observations, not physical host
enrollment or crash-durable publication.

Next bind this same original empty fence to authority/signed expected intent,
then preregister exact no-overwrite reservation publication and its
partial/uncertain/crash behavior. Authenticated peer/clock, reservation and
consumption publication, trusted ownership enrollment and coordinated binary
rollout still precede output/start/watchdog/owned cleanup. All frozen
qualification inputs and the inherited plan-digest refusal remain unchanged.
