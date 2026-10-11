# Fixed diagnostic context and campaign admission

The previous local package proved the short existing flock but retained a
negative case: after its guard dropped, legacy campaign admission ignored fixed
Reserved state. This package closes that read-side gap for the canonical Linux
production campaigns path. Baseline is
`f3d9045f36eba442f52fdd94f9c58bb5f20cbaaa`.

Acquire the existing campaign lock first. Open existing fixed intent storage
through the trusted root-only ancestry reader. Retain its original directories,
the original campaigns directory and exact named/original lock metadata across
admission or recovery. Do not add another lock, create storage or silently
transfer campaign ownership. Pinning a legacy worker-owned lock does not make
that lock root authority or admit a future publisher.

Require the fixed leaf to be empty, not merely lacking active-diagnostic.json.
Any regular file, directory, FIFO, symlink, pending record, terminal state,
historical consumption record or unknown future name blocks admission.
Missing/unsafe storage is an explicit refusal, not absence. Scan from a newly
opened descriptor relative to the original leaf each time; shared directory
offsets must not make a second scan look empty. Never follow a directory entry.
This predicate does not decode state or authorize automatic terminal cleanup.

Check under the original flock before campaign marker handling and again after
marker creation or recovery. A late conflict or original-object failure refuses
the operation and preserves any existing or newly created marker; no retry,
cleanup, adoption or permission repair is performed. A short legacy diagnostic
lock transaction also checks fixed context before returning. Read-only
active_campaign_absent reports conservative presence/unsafe errors, but its
unlocked absence observation is not admission.

Nonproduction canonical roots and non-Linux behavior remain unchanged. Tests use
private factories for modeled nonroot storage in the real acquire/recover
algorithms; no public selector or test promotion enters production. Production
dispatch uses the canonical root, so aliases cannot bypass the fixed check.
Empty safe fixed storage permits the legacy route with its current ownership;
that does not enroll diagnostic publication, whose fence still requires private
trusted campaign ownership. Deployment must use a reviewed common binary:
older binaries do not know this check. No installer or host change is made here.

Test durable presence after short fence drop, acquire/recover/recover_active and
the short lock entry, empty repeat scans, all entry kinds, unsafe/missing storage,
original directory/lock replacement, seeded between-check conflicts, marker
retention and original flock exclusion. Preserve the previous negative case as
historical and fixture-only behavior, not a production bypass claim. Retain
baseline/red/repeated/regression/MSRV/lint/documentation receipts. Frozen release
qualification inputs and the inherited plan-digest refusal remain unchanged.

Next are trusted quiescent ownership enrollment, original authority/intent/state
composition and reservation/consumption publication under the same fence, then
output/start/watchdog/owned cleanup. This read-side change does not prove a
physical host enrollment, publisher serialization, crash recovery or launch.
