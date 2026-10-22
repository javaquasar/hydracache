# Build only module path correction

The first direct development cargo check and clippy invocations returned E0583:
model.rs declared mod controls without a path attribute, so Rust looked for
model/controls.rs rather than the sibling controls.rs. The explicit controls.rs
path attribute was added before either measured test selection. Build-only
check and strict clippy then passed. This note records that development failure;
it is not a retained raw console log or a benchmark observation.

Both first measured compiler selections passed all nine tests. Neither was
rerun, filtered or replaced. Stage/main request conservation is checked offline
from those first logs, without additional measured selections. The counter's
caught unwind-control panic in test output is an intentional passing self-test.

Synthetic Read calls and File tokens do not open a real authority or establish
Linux syscall/descriptor behavior. Original windows, first Causal refusals and
the rejected boot experiment remain unchanged. No live/server/service/clock
operation occurred.
