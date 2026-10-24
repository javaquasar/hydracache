//! Static references from the pre-integration sealed census contract evidence.
//! No Git, current-file read or observed-response-derived reference preparation.
use super::census_caller::{Failure, Reason, Reference};
use super::decoder::{Expected, Kind};

pub const SOURCE: &str = "tools/long-run-supervisor-074/src/diagnostic_artifacts_linux.rs";
pub const BASE: &str = "25faa7bb762aa229631e6f6cfc25cae33e16ff43";
pub const REVIEWED: &str = "c91068bfa8b297bb075b63457da235aaa9c6232e";

pub const ROWS: [(&str, usize, &str); 2] = [
    (
        "2ca72f8386735580ee4cd82ced9a1b07648c9df0",
        34850,
        "ede23782d4ff6a5ae052ca542998c3ebd8968af2ad678f67b179af4fdb05d828",
    ),
    (
        "e4111fdbbb90e0c06a271617da8b3781aa25d58e",
        35108,
        "6b646ab26eff108b751316512dfacff3580cae89dd399366011c8a448fe2ed5f",
    ),
];

pub fn reference(revision: &str, source: &str) -> Result<Reference, Failure> {
    let index = match revision {
        BASE => 0,
        REVIEWED => 1,
        _ => {
            return Err(Failure {
                reason: Reason::Reference,
                output: None,
                original_io: None,
            })
        }
    };
    if source != SOURCE {
        return Err(Failure {
            reason: Reason::Reference,
            output: None,
            original_io: None,
        });
    }
    let (oid, bytes, digest) = ROWS[index];
    let sha256 = std::array::from_fn(|i| {
        u8::from_str_radix(&digest[2 * i..2 * i + 2], 16).expect("fixed SHA256")
    });
    Reference::new(
        &format!("{revision}:{source}"),
        &Expected::Object {
            oid: oid.to_owned(),
            kind: Kind::Blob,
            bytes,
            sha256,
        },
    )
}
