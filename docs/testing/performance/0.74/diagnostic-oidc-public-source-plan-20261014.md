# One public OIDC source acquisition

The operator's 2026-10-14 continuation authorizes the next public-source read
after local acquisition tests, not production key selection, token minting,
protected GitHub configuration, workflow dispatch, installation or server work.
Source is `709d1165f3dc8bfc8aed5630caa58a5a06a14d83`; the acquisition script and
offline inspector are unchanged. Existing historical packets remain immutable.

## Source and runtime inventory

Use Windows Python 3.13.7 and OpenSSL 3.0.16 with the exact inventoried executable,
TLS libraries and source files. The clean preflight loads 69 platform CA roots;
its sorted DER-hash roster digest is
`dc4dfefbd722e067db82e22d9849be097b6701c51979ffe5c7813b86aac24887`.
Retain public metadata/hashes and compare them around the acquisition. This is
local runtime inventory, not physical host or production CA custody enrollment.

The inherited environment contains CURL_CA_BUNDLE. The normal environment guard
refuses before network without reading its value; preserve that preflight
negative. Launch one separately owned clean process, rather than weakening the
guard or mutating the parent environment. Pass only SystemRoot/WINDIR and
temporary-directory/locale names. No proxy, CA, key-log, OpenSSL, Python-path or
GitHub-token setting is passed. Use the reviewed program's ordinary acquire-public
entry with -E/-s; do not call its internal worker directly.

## One attempt and retention

Run exactly one normal acquire-public invocation. Keep the program's fixed two
HTTPS paths, platform chain/hostname/SAN checks, 15-second owned worker deadline,
five-second resource bounds, JSON budgets and refusal behavior unchanged. The
outer launcher has a bounded 20-second wait and may terminate only its exact
owned process. A timeout or failure retains the bounded status and stderr and
stops live acquisition. No retry, fallback address/CA, token or unsafe TLS probe.

On success retain the byte-exact canonical stdout report as public evidence,
including the two base64 raw bodies and their digests. Recompute the offline
inspection independently and compare the peer/TLS/CA and timestamp metadata.
Require every selection/config/job/admission flag to remain false. Retain native
exit status and source/runtime inventory before and after. A received JWKS roster
does not select a kid, snapshot validity, workflow SHA/subject or authority epoch.

If refusal prevents a report, distinguish certificate/transport/representation
or source-profile mismatch only through safe local diagnosis and existing unit
fixtures. Do not remove guards to make the external result green. Preserve the
negative observation and any remaining blocker without claiming actual key
acquisition. Preparation of the dedicated job can continue locally as a design;
no active workflow, permission or secret is installed by this plan.

## Verification and next boundary

Run affected local Python regression and read-only evidence verification,
documentation, registry, format and local performance gates. Retain public
packet hashes and negative outcomes, with no benchmark measurement or release
claim. After source acquisition, separately review real key/workflow/subject/
expiry selection and protected-job custody before any token production. Original
worker policy/epoch, trusted host/root, replay/common fence, authenticated start
and all-writer cessation remain independent prerequisites.
