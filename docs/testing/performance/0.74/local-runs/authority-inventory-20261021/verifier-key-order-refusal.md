# Structural verifier repair

The first `review.ps1 -Mode Seal` exits 1 with `Summary differs from raw
observations`, before writing inventory. It serializes an ordinary Hashtable
of isolated deltas in process-dependent object-key order and compares JSON
strings. Values and captured inputs agree; JSON object order is not a fact.

Keep the original verifier, raw summary and every capture/check unchanged.
`review-structural.ps1` replaces only equality with JsonNode.DeepEquals and
disallows WriteSummary. It self-tests reversed object-key order and refuses
eight value/type/field/case/array-order mutations. All original data, input,
recipe, check-log, scope, conservation, ten scope-mutation and prior-packet
checks remain. No threshold, source or measurement changes; no capture retry.
