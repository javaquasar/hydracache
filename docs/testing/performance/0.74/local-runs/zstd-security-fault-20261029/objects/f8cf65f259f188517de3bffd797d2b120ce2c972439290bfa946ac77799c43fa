# Signed bootstrap component checks

Only static build assertions, retained source/sealed image tests and deterministic
owner models are selected. No original-owner live fixture, executable/bootstrap
start, service, mount, clock, product workload or server is invoked. Fixed lanes
refuse an existing directory; retention verifies exact commands, counts, hashes,
source stability and checked commit, then creates one exclusive inventory.

Run python -B tools/bootstrap-build-owner-074/check.py with one final lane:
host-final, 194-final, 188-final or metadata. Linux uses the existing local nonroot
Ubuntu toolchains. The earlier host/host-v2 lanes retain source-adapter failures;
the earlier Linux lanes retain passing commands but failed source-stability
checks while those adapters were edited. Do not overwrite or promote those lanes.
After
all lanes pass, commit the checked sources and run retain.py --checked-commit SHA.
Do not use static/model success as original-start or server admission.
