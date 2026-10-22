# First broad selection retained as a scope refusal

Legacy source 49f10265 produced one selected test/eight filtered on both compilers.
Candidate source e7304785 unexpectedly produced twelve tests/zero filtered.
Its one-element if-expression selection became System.String, and native splatting
expanded the string into single-character filters. Old allocation/reference and
counter tests therefore executed inside the new test image, contrary to the
preregistered scope. All first output/receipts remain; old sealed evidence was not
overwritten. Unexpected allocation rows and repeated legacy rows are not used as
new results. The independent reader keeps target-isolation REFUSED.

Source 302505d7 uses a typed shared selection helper and pre-launch identity/count
guards. Standalone native argv controls retain both the original character split
and repaired argument vectors. They do not launch a model/test image. No candidate
image was rerun after this fix. The earlier console argv diagnosis is described
here, not represented as an additional retained raw log. Formal first argv logs
and their receipt live under argv/; the echo executable stays only in target/.

The new operator observations cover prechecked logical state/epoch only. Model
publication cuts are not filesystem creation/write/fsync/SIGKILL checkpoints.
No benchmark timing, RSS, copy or admitted production execution result follows.
