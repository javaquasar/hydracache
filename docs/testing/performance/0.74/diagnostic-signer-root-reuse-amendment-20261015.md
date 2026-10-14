# Share pure runtime validation without duplicating the ELF module

Strict Rust1.94 Clippy rejects loading the same ELF file as two modules. Keep
that lint enabled and the parser private. Replace the root fixture's duplicate
module/graph with one pub(super) pure four-image validator in the existing runtime
test module. It accepts only borrowed bytes, reuses the original parser/graph,
returns no metadata/descriptor/owner and provides no custody or launch authority.
The original retained guard and tests remain unchanged in behavior. Root rechecks
use streamed hash equality and this same pure validator on the four bytes.

This preregistered reuse correction does not alter the GNU helper, launch,
seccomp policy, namespace mechanics, workload, bounds, dependencies or closed
admission. The original plan and rejected duplicate-module capture remain retained.
