# Historical source text representation amendment

The first unit setup refused with Qualification drift before running its twelve
tests. Raw comparison establishes that the Git blob and working qualification
file are 2979 LF bytes, while git archive emitted 3068 CRLF bytes. The unchanged
reader requires the original raw LF qualification SHA-256
11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc.
The initial source image and input manifest are retained in the new packet;
initial manifest canonical SHA-256 is
06d27984f53d0a4d02d1cdeb76412e2e7e1dd3b9eb953f739fc9d2e9dc769c01.

All 129 discovered working source images are UTF-8 LF without BOM and their raw
hashes equal the recorded canonical hashes. Freeze a second input-manifest
identity specifying that exact representation for every source file. This is a
source-file text representation adapter,
not a change to qualification, signed records, evidence bytes or semantics.
Verify archived canonical content before reconstruction, verify the resulting
raw bytes before importing readers, requiring the same pinned digest at both
boundaries. Evidence packets
remain byte-exact and never pass through this adapter. Unknown representations
or mismatched raw hashes refuse. No reader, source predicate, expected SHA,
deadline or original first refusal may be relaxed. The initial explanation
reversed LF and CRLF; that inference is superseded by the raw byte comparison,
not by a changed expected qualification hash.
