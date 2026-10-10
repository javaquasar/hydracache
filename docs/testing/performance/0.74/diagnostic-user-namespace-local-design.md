# Original process and reader user namespace observation

## Local scope and sequence

Baseline `bde3a8ac4ee00620672d6e2cd05c2471bec03bc7`. Add an opt-in Linux
read-only user namespace guard over an already retained original `ProcessRead`.
Record baseline, missing API check, tests, implementation and repeated local
results separately. No PID, namespace ID, pathname, FD, account or numeric policy
is accepted from the caller. The existing credential/account APIs are unchanged.
Use write-page for repository design/article prose; no cloud Page is created.

## Namespace identity and observations

Open `ns` relative to the original process directory without following symlinks.
Only the fixed `user` namespace magic link is intentionally followed, using
read-only close-on-exec descriptors. Require nsfs and `NS_GET_NSTYPE` equal to
`CLONE_NEWUSER`, then compare device/inode identities while retaining both
namespace descriptors. Read the observer through the fixed `thread-self` proc
magic link, not the process leader's `self` namespace: the reading thread is
the subject of this check. Validate its directories against the retained procfs
root. No string-form namespace identity or caller assertion replaces kernel IDs.

The [namespace type API](https://man7.org/linux/man-pages/man2/ns_get_nstype.2const.html)
returns the namespace's CLONE_NEW type. The
[user namespace mapping rules](https://linuxman7.org/linux/man-pages/man7/user_namespaces.7.html)
make numeric ID interpretation depend on the reader namespace. Equality here
establishes only the same retained user namespace; it does not establish the
initial/host namespace, a trusted provider or an admitted numeric mapping.

Require the process namespace to match the observer's original namespace before
returning a guard. Revalidate original process generation, namespace, generation,
namespace, generation. Each namespace step checks retained descriptors, the
original process's named namespace directory and namespace, and the current
reader namespace before/after that named observation. Every failure permanently
refuses this guard; later restoration cannot refresh it. No FD/namespace token
export, reset, setns/unshare, writer revocation or execution capability exists.
This standalone guard does not latch other guards and is not yet composed with
the credential reader. Credential-file opener namespace must be bound explicitly
in that later composition; a guard captured afterwards cannot retroactively
certify the view used when status was opened.

## Tests and proof boundaries

Test exact observation order, every failing step, typed first error, mismatch,
prior refusal, valid seeded identity mutations, and independent concurrent guards.
Real owned non-product cat helpers cover repeated same-namespace positives,
original process exit, namespace descriptor substitution/restoration and safe
descriptor flags. Real mount namespace and ordinary file descriptors must refuse
as user namespaces. Foreign-user-namespace drift tests are synthetic: no test
creates or joins a namespace or changes a host account/unit.

Trusted host/NSS policy, initial namespace and all-thread proof, credential-opener
binding, production output preparation, authenticated original start, watchdog
recovery and a positive lifecycle rehearsal remain separate. No helper/IPC route,
host install, product workload or qualification is enabled. Frozen qualification
inputs, thresholds and closed ship admission remain exact.
