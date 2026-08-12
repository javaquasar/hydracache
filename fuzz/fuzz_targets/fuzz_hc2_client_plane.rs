#![cfg_attr(fuzzing, no_main)]

#[cfg(fuzzing)]
use hydracache_client_hc2::wire::{ClientEnvelope, ServerEnvelope};
#[cfg(fuzzing)]
use hydracache_client_plane_spike::fault_proxy::FaultReplayArtifact;
#[cfg(fuzzing)]
use hydracache_client_plane_spike::{SpikeCodec, TransportCandidate};
#[cfg(fuzzing)]
use prost::Message;

#[cfg(fuzzing)]
libfuzzer_sys::fuzz_target!(|data: &[u8]| {
    // Generated protobuf decoders must reject or decode arbitrary input without
    // panicking. The owned framing codecs must do the same for every candidate,
    // and retained fault artifacts must stay parser-safe under mutation.
    let _ = ClientEnvelope::decode(data);
    let _ = ServerEnvelope::decode(data);
    for candidate in TransportCandidate::ALL {
        let _ = SpikeCodec::new(candidate, 64 * 1024).decode(data);
    }
    let _ = serde_json::from_slice::<FaultReplayArtifact>(data);
});

#[cfg(not(fuzzing))]
fn main() {}
