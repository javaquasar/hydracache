// A finite order model, not a staged executor or a benchmark. The first SET
// has already committed. A fixed, fully encoded batch contains both replies;
// its write begins before the first reply's flush completes. It has no dynamic
// mutation callback and may expose all supplied bytes to the writer immediately.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Event {
    BeginFixedBatch,
    FlushFirstReply,
    CommitSecondSet,
}

fn preserves_order(first_flush: usize, second_commit: usize, second_write: usize) -> bool {
    first_flush < second_commit && second_commit < second_write
}

#[test]
fn fixed_two_reply_batch_cannot_preserve_both_commit_frontier_and_response_order() {
    use Event::{BeginFixedBatch as B, CommitSecondSet as C, FlushFirstReply as F};
    let schedules = [
        [B, F, C],
        [B, C, F],
        [F, B, C],
        [F, C, B],
        [C, B, F],
        [C, F, B],
    ];
    let mut chronological_batch_schedules = 0;
    let mut admitted = 0;
    for schedule in schedules {
        let index = |event| {
            schedule
                .iter()
                .position(|candidate| *candidate == event)
                .unwrap()
        };
        let begin = index(B);
        let flush = index(F);
        let commit = index(C);
        if begin >= flush {
            continue; // A flush cannot complete before this batch begins.
        }
        chronological_batch_schedules += 1;
        if preserves_order(flush, commit, begin) {
            admitted += 1;
        }
    }
    assert_eq!(chronological_batch_schedules, 3);
    assert_eq!(admitted, 0);

    // Positive control: the canonical path finishes the first write+flush,
    // commits SET 2 and only then starts its separate response write.
    assert!(preserves_order(0, 1, 2));
    assert!(!preserves_order(1, 0, 2), "early second mutation");
    assert!(!preserves_order(0, 2, 1), "early second response");
}
