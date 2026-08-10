# HC/2 Resource Ownership Ledger

HC/2 refuses invalid zero limits before a connection reaches `Ready`. Every
retained allocation class has one named owner, a nonzero configuration bound,
privacy-safe accounting, an atomic one-over refusal, and deterministic release.

| Retained class | Owner | Bounds | Release |
| --- | --- | --- | --- |
| inbound/message frame | codec | encoded bytes | after decode/dispatch |
| batch | decoder | item count | after validation/dispatch |
| pending invocation | connection | count | completion, cancellation, close |
| reply queue | reply lane | frame count and encoded bytes | drain or close |
| event/gap queue | event lane | frame count and encoded bytes | drain, gap replacement, close |
| control queue | control lane | encoded bytes | drain or close |
| retry record | invocation runtime | count | terminal attempt or close |
| reconnect record | connection runtime | count | terminal reconnect or close |
| deadline | connection timer set | count | completion/cancel/expiry/close |
| subscription | connection | count | unregister or close |
| topology | connection view | node count and encoded bytes | atomic replacement or close |
| session | connection generation | count and state bytes | loss/close/disconnect |
| admitted connection | shared admission ledger | identity, tenant, global count | RAII permit drop |

Reply completion checks both queue bounds before removing pending work. Topology
replacement and session admission check all relevant bounds before mutation.
Event overflow conservatively replaces retained history with exactly one bounded
gap. Admission checks identity, tenant, and global scopes under one lock before
incrementing any scope. `disconnect` clears every connection-owned class and
must return the all-zero `ResourceSnapshot`.

The current spike deliberately exposes stable static overload class names rather
than payload, key, identity, or tenant values. H21 maps these classes into the
bounded `hydracache.hc2.client_plane.v1` diagnostic contract. H01 still owns
connecting that contract to the selected production listener and exporter.
