//! Pure bounded collection only; no proc reads, clock or original owner.
use super::*;

const LENGTHS: [usize; 7] = [0, 1, 4, 5, 16, 32, 33];
const ITERATIONS: u64 = 256;

#[test]
fn projection_groups_reference_requests() {
    for length in LENGTHS {
        let input = (0..length)
            .map(|n| n.to_string())
            .collect::<Vec<_>>()
            .join(" ");
        let (_, counts) = capture(|| {
            for _ in 0..ITERATIONS {
                let values = std::hint::black_box(input.as_str())
                    .split_ascii_whitespace()
                    .take(33)
                    .map(|v| v.parse::<u32>())
                    .collect::<Result<Vec<_>, _>>()
                    .unwrap();
                assert_eq!(std::hint::black_box(values.as_slice()).len(), length);
            }
        });
        assert_eq!(counts.allocations, if length == 0 { 0 } else { ITERATIONS });
        println!(
            "HC074-PROJECTION-GROUPS-REFERENCE {}",
            serde_json::json!({
                "length":length,"iterations":ITERATIONS,"counts":counts,
                "rust_requests_only":true,"profile":false,"server":false,"product":false
            })
        );
    }
}

#[test]
fn projection_groups_candidate_requests() {
    use crate::diagnostic_kernel_admission::projection_groups::Values;
    for length in LENGTHS {
        let input = (0..length)
            .map(|n| n.to_string())
            .collect::<Vec<_>>()
            .join(" ");
        let (_, counts) = capture(|| {
            for _ in 0..ITERATIONS {
                let values = std::hint::black_box(input.as_str())
                    .split_ascii_whitespace()
                    .take(33)
                    .map(|v| v.parse::<u32>())
                    .collect::<Result<Values, _>>()
                    .unwrap()
                    .into_vec();
                assert_eq!(std::hint::black_box(values.as_slice()).len(), length);
            }
        });
        assert_eq!(counts.allocations, if length == 0 { 0 } else { ITERATIONS });
        assert_eq!(counts.reallocations, 0);
        assert_eq!(counts.requested_bytes, 4 * length as u64 * ITERATIONS);
        println!(
            "HC074-PROJECTION-GROUPS-CANDIDATE {}",
            serde_json::json!({
                "length":length,"iterations":ITERATIONS,"counts":counts,
                "rust_requests_only":true,"profile":false,"server":false,"product":false
            })
        );
    }
}
