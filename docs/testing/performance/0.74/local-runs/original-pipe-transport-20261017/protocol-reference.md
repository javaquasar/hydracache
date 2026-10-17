# Unsent message representation

The upstream systemd v255 service property handler accepts
StandardOutputFileDescriptor and StandardErrorFileDescriptor. Its systemd-run
client encodes these properties as D-Bus type `h`, not an integer property.
See the [service handler](https://github.com/systemd/systemd/blob/v255/src/core/dbus-service.c#L623-L629)
and [client construction](https://github.com/systemd/systemd/blob/v255/src/run/run.c#L762-L767).
This packet uses the already locked zbus 5.19.0 and zvariant 5.15.0 FD wrappers.
Its tests inspect actual encoded bodies and their owned out-of-band FD table.

The read-only host version query reports systemd 255 (255.4-1ubuntu8.17).
The upstream tag is a protocol reference, not a vendor-source attestation or a
successful installed manager call. No bus connection, SCM_RIGHTS delivery,
manager acknowledgement, worker execution or writer revocation is exercised.
Those require the separately gated authenticated start and armed watchdog.
