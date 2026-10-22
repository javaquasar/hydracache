# Audit publication directory prerequisite

The first audit CLI completed its read-only analysis but refused publication
because the new evidence directory had not been created. Python reported
FileNotFoundError for authority-integration-audit-20261022/facts.json through the
exclusive publish helper. No facts, existing packet or source was overwritten.

The operator then created that exact new directory and published the facts
without changing source, assertions or inputs. The twelve source-contract tests
had already passed. This was an output-directory prerequisite failure, not an
executed model/live/fault attempt or a performance retry. Final verification
receipts describe their own subsequent first check cohort.
