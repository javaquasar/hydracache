# Isolated original-manager diagnostic fixture

This is a manual local WSL gate, not a production installer or admission tool.
The exact scope and first failures are in the repository's
[report](../../docs/testing/performance/0.74/bootstrap-original-service-20261023.md).

check.py runs only offline/component checks. Its baseline is already consumed;
never rerun it. No check lane selects --ignored. GNU 1.94 and MSRV 1.88 are built
in Ubuntu caches; the privileged readiness fixture belongs only to the separate
HydraCache074Native distro. A new native execution requires the recorded user
permissions, fresh absence checks and a separately versioned single-use lane.
The current native output path is consumed and must not be deleted to retry.

native_fixture.py records and fsyncs the intent through a writable exclusive
handle, copies only diagnostic artifacts and calls one bounded shell session.
native_fixture.sh creates only the approved temporary account/groups and fixed
diagnostic bootstrap. Test enrollment stays in the same private trust root as
the consumed owner. No production key/enrollment, clocks, masks, existing service
or server are modified. A failed test remains failed even if cleanup succeeds.

Account recovery copies use root-only /var/tmp, not WSL's per-init tmpfs /tmp.
Only exact inode/digest-matching installed files and marked newly created units
may be retired. Unknown lookup states refuse; systemctl show is not an absence
test for slices. Cleanup must never be presented as all-writer retirement proof.

Run offline controls with:

    python -B -m unittest discover -s tools/bootstrap-original-service-074 -v

seal.py retain builds a new exclusive content-addressed evidence archive from
the fixed check/native lanes. seal.py verify validates its objects without
contacting a manager, starting a service or changing any host state. The packet
is intentionally non-admitting and retains both preparation and original-test
first refusals. It excludes binary copies and private account recovery contents.
