# Exact-view authoring refusal

The first Before xtask check is retained with exit 101. The new adapter correctly
requires a known complete inventory, but initially recognizes only current and
proposed bodies. An unchanged older assertion deliberately restores the exact
historical materialized-set body and checks idempotence; the new layer refuses
that known historical input. Add only the already frozen historical template
to the explicit accepted inputs and test it in the new canary. Unknown bodies,
changed guards, duplicates and unknown helpers remain refused. No old assertion
is edited and no measured invocation has occurred. Retain the original recipe,
failure receipt and log; use a distinctly named repair check recipe and files.
