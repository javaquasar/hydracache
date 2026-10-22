# Offline guard development failure

The first direct development invocation of test_audit_read.py ran eleven tests:
ten passed and test_regular_file_flags_and_all_stamp_fields_are_preserved failed
with AssertionError: ValueError not raised. Its mutation removed a Stamp field
declaration, but the auditor's substring search also matched that field in the
constructor. The guard was corrected to search only the Stamp declaration,
while the independent constructor-observation checks remain.

The next direct development invocation passed all eleven tests. The first
immutable closing recipe also passed those eleven and the preceding thirty-six
offline controls, forty-six Windows source canaries and structural checks.
This note is a development record, not a retained raw log or a kernel fault
observation. No authority/runtime source, old outcome or admission window was
changed, and no measurement or live attempt was repeated.
