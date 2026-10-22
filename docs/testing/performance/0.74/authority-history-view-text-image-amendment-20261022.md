# Historical source text representation amendment

The first unit setup refused with Qualification drift before running its twelve
tests. Git text normalization produced LF source bytes, whereas the unchanged
reader requires the original raw CRLF qualification SHA-256
11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc.
The initial source image and input manifest are retained in the new packet;
initial manifest canonical SHA-256 is
06d27984f53d0a4d02d1cdeb76412e2e7e1dd3b9eb953f739fc9d2e9dc769c01.

Freeze a second input-manifest identity recording canonical source hashes plus
the exact original raw image hash and LF/CRLF representation for every one of
the same 129 source files. This is a source-file text representation adapter,
not a change to qualification, signed records, evidence bytes or semantics.
Verify archived canonical content before reconstruction, verify the resulting
raw bytes before importing readers, and retain both hashes. Evidence packets
remain byte-exact and never pass through this adapter. Unknown representations
or mismatched raw hashes refuse. No reader, source predicate, expected SHA,
deadline or original first refusal may be relaxed.
