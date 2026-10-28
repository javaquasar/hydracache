# Exact vendored Swagger asset review

This is a local read-only source/input reviewer, not an extractor or a new
production admission mechanism. It checks the exact crates.io archive for
utoipa-swagger-ui-vendored 0.2.0 and the original embedded Swagger UI 5.32.6 ZIP,
then independently reads every entry and CRC. Path aliases/traversal, symlinks,
special files, privileged bits, duplicate entries, file/parent conflicts,
encryption, unsupported compression and fixed review limits refuse. No files are
extracted and no archived code is executed. A passing fixed input does not prove
safety for arbitrary untrusted ZIPs or a hostile preexisting build output tree.

```
python -B -m unittest discover -s tools/swagger-assets-review-074 -p test_*.py -v
python -B tools/swagger-assets-review-074/review.py <exact-0.2.0.crate> <new-output>
```

Cargo.lock must separately bind the same registry checksum. The sandbox keeps
utoipa 5 generation and its existing typed JSON handler; only the UI consumes
utoipa 6. HTTP regression tests compare the full OpenAPI response, authorization,
local assets, initializer URL and refusal behavior before and after migration.
`default-features = false` and `vendored` avoid the upstream downloader path;
inherited `SWAGGER_UI_*` build overrides are not part of the reviewed candidate.
