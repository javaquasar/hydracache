# Concrete shipping identity decision, not an applied Root change

Both compiler controls now establish the same result: three runtime positives
and three exact nominal compile refusals per Rust1.94/1.88. Configuration and
borrowed ID return E0308; old ChannelTransport against the owned Transport returns
E0277. Both real packages display the same Rust library paths, but their TypeIds
and public nominal boundaries differ. This does not compile the full Root adapter
or prove complete wire/runtime/performance compatibility.

The [exact unapplied change](chitchat-public-identity-migration-proposal-20261030.json)
records whole before/after hashes and one unique workspace dependency replacement:

```toml
chitchat = { package = "hydra-chitchat", version = "=0.11.1-hydra.1" }
```

The four direct workspace consumers already inherit this single alias. Preparing
a copied-Root candidate with this binding could keep their Rust source unchanged,
but needs actual adapter/testkit/raft/sandbox coverage. No full Root compilation
has been performed here and the real manifest/lock remain unchanged.

For an external typed caller currently declaring `chitchat = "0.11"`, accepting
the maintained route requires coordinating its Cargo dependency to the same exact
owned package/version. Existing `use chitchat::...` spelling can remain; values
from the independently retained legacy package cannot cross the three boundaries.
Callers that still need the legacy library elsewhere must keep it under a separate
alias and never treat the two nominal families as interchangeable. This is an
explicit public dependency migration, not transparent compatibility. No wrapper,
transmute, serialization bridge or public Rust method change is proposed.

Recommended decision: accept this coordinated identity migration as the path for
preparing the eventual Root candidate. It uses the exact already-qualified
80,206-byte maintained artifact0296600c and can remove the old security dependency
without transferring runtime costs through a compatibility bridge. Numerical
performance and complete Root compatibility still require their existing gates.
The alternative is to require the original crates.io package identity; the owned
fork then cannot satisfy that condition, and an appropriate upstream release is
needed. No upstream publication availability is assumed.

Neither choice authorizes package publication, actual Root adoption/lock edits,
server installation or any waived check. First obtain the operator's identity
decision, verify a coherent copied-Root caller candidate, and independently meet
publication/public-registry, explicit MTU-scope and exact Root security/package/
workspace/platform admission requirements. The public name query's observed404
does not prove ownership or reserve the coordinate.
