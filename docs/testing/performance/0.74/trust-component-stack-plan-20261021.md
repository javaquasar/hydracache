# Short owned trust component C string candidate

Test one storage-only hypothesis against the sealed CString baseline
`a72457aade616e67d2fc5693f6105c8063d6e9e1`. Use private invocation-local owned
32-byte storage for names shorter than 32 bytes; keep CString construction for
all other names, including generic, Unicode and slash/dot-containing inputs.
Short embedded-NUL names return the same Invalid before open without allocating
an owned NulError payload that the child helper discards. Long refusals retain
CString fallback. Do not add a component/path validator or change error class.

Replace only the child name constructor and its now-unused CString import,
with exact private module/test hooks. Keep flags, original parent, pointer to
identical terminated bytes, one openat, negative-fd/last-error/File ownership
handling and no EINTR retry. All fresh checks, call counts/order and original
300/60/10-second windows remain. No new unsafe, cache, retained observation,
public representation or borrowed origin is introduced. The 32-byte array is
not the total local type/stack footprint.

Compare the same 21 valid and four NUL baseline shapes, 256 constructions each,
against the actual helper through a pure test adapter. Repeat the unchanged
baseline and counter controls. Freeze complete old/new child-body models and
compare the same 512 seeded schedules plus explicit first-error, long-NUL,
boundary, pointer and independent ownership controls. Whole-source guards must
reverse the exact constructor/import/hooks, including within historical tests;
no filename exemption or removal of live predicates is permitted.

Run only pure units/source canaries, affected all-target check/strict all-feature
lint, formatting and local structural gates. Preserve first failures/logs; use
a separate cohort for any test repair. Verify eight prior raw packet seals and
348 members without changing frozen reviewers. No ignored/manual/live profile,
clock/service change, server/SSH/Actions/install/supervisor/product operation.

Keep the candidate only if its allocation and semantic controls pass; otherwise
retain the negative result and revert this leaf only. This work does not time
full preparation, prove material/cleanup/delivery/all-writer cessation, or open
SIGKILL/lost-ACK/reconnect/server gates. Both first Causal(Time) failures remain.
