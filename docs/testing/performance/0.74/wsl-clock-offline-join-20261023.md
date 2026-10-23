# Offline audit of clock and NTP publication order

The original 81-row analysis reproduces exactly from its retained archive.
An additional offline join accounts for the fact that each clock sample precedes
its NTPMessage getter. It preserves the evidence for opposing corrections without
pretending that a getter and a clock sample are simultaneous. It cannot identify
the backward setter or the historical writer of tick. No new WSL observation,
original-owner execution, trace configuration or service action was performed.

## Publication geometry

The saved shell reads clocks for row i, then queries NTPMessage, then sleeps
before the next row. A newly observed packet may therefore have been processed
after row i's clock read but before its getter. The complete surrounding clock
window is rows i-1 through i+1, assuming the upstream callback successfully
applies its correction before publishing the saved response. A missing following
clock, packet-count gap or ignored spike cannot be converted into an applied
step. Small offsets also cannot be subtracted as instantaneous full steps: the
[upstream 255.4 path](https://raw.githubusercontent.com/systemd/systemd-stable/v255.4/src/timesync/timesyncd-manager.c)
uses a slew below its existing NTP_MAX_ADJUST value of 0.4 seconds. This analysis
does not change that constant or any HydraCache threshold.

The exact integer arithmetic over the wider windows gives:

| Packet | Surrounding clock rows | Net REALTIME minus MONOTONIC delta ns | NTP offset from saved timestamps ns | Remainder if the NTP step was applied ns |
| --- | --- | ---: | ---: | ---: |
| 308 | 10 through 12 | 2730957131 | 8785012500 | -6054055369 |
| 309 | 39 through 41 | 8786289453 | 8786289500 | -47 |
| 310 | 67 through 69 | 2727654980 | 8786406000 | -6058751020 |

The two large negative remainders remain after accommodating publication order.
They support the opposing-correction model under the stated application
assumption. They are not exact syscall arguments, event timestamps or caller
identities. The -47 ns arithmetic remainder for packet 309 is not a nanosecond
accuracy claim: the exposed timestamps have microsecond representation, clock
reads are sequential, and the installed binary has not been traced. Both exact
setter fields remain null. The original single-interval results remain unchanged.

## Scope and evidence

offline_join.py reads only the pinned content-addressed archive, validates its
index and every object, pins the decoder/method source bytes and reproduces all
original analysis fields and session identities. It neither calls the live
collector nor enables tracing. Its output is a new exclusive analysis, not a
rewrite of any first result or a new admission decision.

Twenty-two offline controls pass, including publication after the same row's
clock read, opposing remainders, missing future observations, packet gaps,
spikes, slew-versus-step handling, unchanged original guards and tampered
archive rejection. The original source archive index remains
798b204c4b14865f482d64cdc4176aec7c1c3c4126b87bf635e8d9c2ba140082.
Original-owner failure and qualification anchors are unchanged; all 300/60/10
windows and live runtime/fixture sources remain fixed.

The separate offline packet retains 28 logical files in 25 exact SHA-256 objects,
including this analysis method, controls, result, source input index and check
receipts. Its index is
56250b03ab2aab0efb3cd1d5e559fcae6f5e0e64314f75693297d7ef4bff8aff.
doc-check, the registry of 188 gates and the local non-promotable 0.74 contract
pass. No original archive was resealed. The analysis implementation is committed
as 1755fad4; evidence and narrative are separate follow-up commits.

## Required boundary before caller tracing

Enabling even a separate tracefs instance changes observation state and is not
part of a strictly read-only investigation. The existing read-only limit remains
in force until observation-state changes are explicitly authorized. Before a trace, freeze
the target symbols/ABI, allowed fields, duration, buffer cap, loss accounting,
cleanup and original global-state comparison. Use only uniquely owned temporary
objects and leave existing probes/events/instances untouched. A controller loss
must not leave tracing enabled indefinitely.

A future trace may identify a newly observed writer or driver path. It cannot
recover who set tick before tracing started if no new tick write occurs. Empty
or incomplete capture, missing ABI support, lost records or unconfirmed cleanup
must remain inconclusive, not trigger an automatic repeat, relaxed clock check
or original-owner execution. Clock and service changes and server actions remain
outside the proposed observation-only scope.
