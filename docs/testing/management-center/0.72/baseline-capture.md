# Management Center 0.72 external baseline capture

This evidence-only workflow preserves the already qualified 0.72 product candidate. The capture
tool lives on `evidence/0.72-baseline-capture`; it does not alter or rebuild the candidate source.
The candidate, pre-feature source and published `v0.71.0` source are separate clean checkouts.

The workflow rejects a symbolic or dirty substitution. It resolves the pre-feature commit and the
annotated `v0.71.0` tag through the candidate repository, compares those commits with the baseline
checkouts, and binds every receipt to the exact candidate SHA. Raw measurement details are retained
beside each compact receipt. The compact receipt contains SHA-256 digests of the capture command,
tool revision and each raw measurement object.

The measurements have these fixed meanings:

- `legacy_console_tests`: number of successful Playwright project/test instances in the published
  0.71 console suite;
- `bundle_bytes`: total bytes of the shipped top-level HTML, JavaScript and CSS assets;
- `endpoint_latency_bytes`: mean of five complete live-fixture page observations in milliseconds;
  the paired raw record retains the exact response bytes;
- `server_retained_owners`: positive file-descriptor plus task growth in the console origin process
  after five browser observations and one second of quiescence;
- `browser_heap_dom`: Chromium used JavaScript heap bytes after the final observation; the paired raw
  record retains the DOM node count;
- `fd_task_counts`: the origin process file-descriptor and task count after quiescence;
- `console_coverage`: executed JavaScript byte-range percentage reported by Playwright.

The capture requires Linux `/proc`, Chromium, Node 22, successful legacy console tests, clean source
checkouts and a clean candidate. A missing browser metric, non-finite value, wrong unit, empty sample
set, ref mismatch, dirty tree or failed legacy test aborts before a receipt is written. The workflow
uploads receipts and raw records as one immutable artifact. The artifact is then materialized under
`target/release-evidence/management-center/0.72/` in the qualified candidate before running:

```text
cargo xtask management-center-check --release 0.72 --require-evidence
cargo xtask release-evidence --release 0.72 --receipts-dir target/release-evidence/receipts --require-ship
```

This is evidence acquisition, not a substitute candidate. Product, threshold, soak, fault schedule,
TTL and load parameters remain those already exercised by the qualified SHA.

Omitting `--receipts-dir` requests a structural report with no runtime receipts; it is not the
release-closing invocation.

