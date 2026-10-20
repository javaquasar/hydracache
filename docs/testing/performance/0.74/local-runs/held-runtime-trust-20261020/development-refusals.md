# Development observations, not accepted-source results

The baseline Windows held_build selection passed 28 controls before the module
hook. missing-module-red.log records the expected missing source-file refusal.
The first implementation compile additionally refused three E0609 accesses:
preset_id on consent/request (the field is preset), and starting_state on the
placement (the field is nested in consent). That console-only failed compile
was not retained as a raw log and is not an acceptance result.

initial-tests.log records 20/22 controls passing. Both failures were test-fixture
expectations, not relaxed product checks. The unchanged shared Snapshot contract
permits 0640 (group read), so the unsafe-write case was corrected to 0660.
After metadata drift to 0400, the unprivileged fixture must restore 0600 before
writing its original bytes; that restoration still cannot revive the refused
input. expanded-tests.log records all 26 controls passing before source freeze.
Final acceptance uses separately frozen source, recipes and actual image hashes.
