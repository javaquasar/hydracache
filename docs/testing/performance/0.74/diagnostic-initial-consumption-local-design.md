# Initial intent consumption under the original fence

This local library slice starts from `175d2dae1509d1b664efe0852074665cc99b82b7`.
Consume the runtime-acknowledged fixed Reserved publisher by value, retaining
its original authority/intent borrow, original storage and uninterrupted lock.
A readable Reserved file, recovered history or fixture authority cannot create
this acknowledgement. This is cell one only, not state advancement, admission
or permission to start a worker. Cells two through four need a separate contract.

Preflight the complete unchanged initial state and original signed intent before
mutation. Reuse the existing retained-intent canonical record schema and digest
marker, with no previous record. Create only `diagnostic-intent-cell-1.json` and
`diagnostic-intent-cell-1.commit`, relative to the original fixed leaf, exclusively
and without following links. Retain each created descriptor. Write bounded bytes,
seal only these new owned files from mode 0600 to 0400, sync both files and the
original directory, and acknowledge only after the original objects, state,
namespace, authority and intent have passed the final bracket.

Keep namespace transitions private and exact: Reserved permits only the original
state; consumption permits exactly that state plus objects actually created by
this attempt, not arbitrary allowlists or preexisting targets. At every private
checkpoint inspect original/named inode identity, owner/group/device, link count,
mode, bytes, ancestry, original common lock and legacy context. Existing targets,
unknown entries, substitutions, hard links and signed-intent drift refuse.

An error owns the publisher and lock until drop, retains partial and synced bytes,
and cross-latches bound originals. Use the independently exact-safe original
authority Policy-refusal bridge, preserving first error and uncertainty without
duplicating an already handled intent refusal. Refused observations perform no
further I/O. Never overwrite, adopt, repair, clean up or automatically retry.

Retain baseline, an absent-public-API red test, checkpoint failures (seed `0x074c`),
three identical focused runs using four independent test threads, relevant Linux
and Windows regression, current/MSRV checks, lint, format and documentation.
Injected checkpoint errors are logical interruptions, not actual SIGKILL or
power-loss proof. Owned-process kill testing is the next separate slice.
Peer and time remain externally asserted. Production wiring, trusted host
enrollment, output/start/watchdog/cleanup and host rehearsal remain closed.
Do not modify frozen qualification inputs or claim product performance gains.
