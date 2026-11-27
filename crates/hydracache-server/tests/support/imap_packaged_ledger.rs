//! Independent replay of the two packaged load clients' submitted batches and receipts.
use std::ops::Range;

const ABSENT: u8 = 1;
const SEED: u8 = 2;
const UPDATED: u8 = 4;

#[derive(Clone, Copy, Debug, Default, serde::Serialize)]
pub(super) struct Counts {
    pub confirmed: usize,
    pub rejected: usize,
    pub not_applied: usize,
    pub unknown: usize,
}

#[derive(Debug)]
pub(super) struct Round<'a> {
    pub keys: [&'a str; 2],
    pub allowed: [u8; 2],
    pub counts: Counts,
    pub start: u64,
    pub end: u64,
}

impl Round<'_> {
    pub fn complete(&self) -> bool {
        self.counts.confirmed == 6 && self.counts.rejected == 1
    }

    pub fn in_window(&self, window: &Range<u64>) -> bool {
        self.complete() && window.contains(&self.start) && window.contains(&self.end)
    }
}

pub(super) fn admits(allowed: u8, value: Option<&[u8]>) -> bool {
    value_bit(value).is_some_and(|observed| observed & allowed != 0)
}

fn value_bit(value: Option<&[u8]>) -> Option<u8> {
    match value {
        None => Some(ABSENT),
        Some(b"seed") => Some(SEED),
        Some(b"updated") => Some(UPDATED),
        _ => None,
    }
}

pub(super) fn replay<'a>(
    input: &'a str,
    plan: &str,
    partitions: &[[u32; 2]],
    sealed: bool,
) -> Result<Vec<Round<'a>>, &'static str> {
    if input.len() > 512 * 1024 || (sealed && !input.ends_with('\n')) {
        return Err("invalid ledger size or incomplete final line");
    }
    let complete = input.rsplit_once('\n').map_or("", |(prefix, _)| prefix);
    let lines: Vec<_> = complete.lines().collect();
    let pairs = lines.chunks_exact(2);
    if sealed && !pairs.remainder().is_empty() {
        return Err("submission without an outcome");
    }
    let mut rounds = Vec::new();
    let mut previous_end = 0;
    for (seq, pair) in pairs.enumerate() {
        let [submit, outcome] = pair else {
            unreachable!()
        };
        let submitted: Vec<_> = submit.split('\t').collect();
        let results: Vec<_> = outcome.split('\t').collect();
        let ["submit", sent_seq, sent, a, b] = submitted.as_slice() else {
            return Err("invalid submission");
        };
        let ["outcome", result_seq, received, items @ ..] = results.as_slice() else {
            return Err("invalid outcome");
        };
        let start: u64 = sent.parse().map_err(|_| "invalid submit time")?;
        let end: u64 = received.parse().map_err(|_| "invalid outcome time")?;
        let expected = plan.lines().nth(seq).and_then(|row| row.split_once('\t'));
        if sent_seq.parse::<usize>().ok() != Some(seq)
            || sent_seq != result_seq
            || start < previous_end
            || end < start
            || expected != Some((*a, *b))
            || a == b
            || items.len() != 7
        {
            return Err("ledger order, timing, plan or item count mismatch");
        }
        let partition = partitions.get(seq).ok_or("missing planned partitions")?;
        let mut keys = [ABSENT; 2];
        let mut counts = Counts::default();
        for (index, item) in items.iter().enumerate() {
            let key = usize::from(index >= 4);
            let old = keys[key];
            if *item == "n" {
                counts.not_applied += 1;
                continue;
            }
            if *item == "u" {
                counts.unknown += 1;
                keys[key] |= match index {
                    0 | 4 => SEED,
                    2 if old & SEED != 0 => UPDATED,
                    5 => ABSENT,
                    _ => 0,
                };
                continue;
            }
            let fields: Vec<_> = item.split(':').collect();
            let [state, part, epoch, version] = fields.as_slice() else {
                return Err("missing certified item proof");
            };
            if part.parse::<u32>().ok() != Some(partition[key])
                || !epoch.parse::<u64>().is_ok_and(|epoch| epoch > 0)
                || !version
                    .parse::<u64>()
                    .is_ok_and(|version| version > 0 || matches!(*state, "0" | "r"))
            {
                return Err("invalid certified item proof");
            }
            if *state == "r" {
                counts.rejected += 1;
            } else {
                counts.confirmed += 1;
            }
            keys[key] = match (index, *state) {
                (0 | 4, "a") => SEED,
                (1, "r") => old,
                (2, "a") if old & SEED != 0 => UPDATED,
                (2, "r") => old & !SEED,
                (5, "a") => ABSENT,
                (3 | 6, "0") => old & ABSENT,
                (3 | 6, "s") => old & SEED,
                (3 | 6, "v") => old & UPDATED,
                _ => return Err("wrong item result for operation"),
            };
            if keys[key] == 0 {
                return Err("confirmed outcome conflicts with journal history");
            }
        }
        previous_end = end;
        rounds.push(Round {
            keys: [a, b],
            allowed: keys,
            start,
            end,
            counts,
        });
    }
    Ok(rounds)
}

#[cfg(test)]
mod tests {
    use super::*;

    const PLAN: &str = "61\t62\n";
    const SUBMIT: &str = "submit\t0\t5\t61\t62\n";
    const OUTCOME: &str =
        "outcome\t0\t8\ta:4:2:1\tr:4:2:1\ta:4:2:2\tv:4:2:2\ta:6:2:1\ta:6:2:2\t0:6:2:2\n";

    #[test]
    fn replay_fences_confirmed_values_and_deletions() {
        let body = format!("{SUBMIT}{OUTCOME}");
        let round = replay(&body, PLAN, &[[4, 6]], true).unwrap().pop().unwrap();
        assert!(round.in_window(&(4..9)) && !round.in_window(&(5..8)));
        assert!(admits(round.allowed[0], Some(b"updated")) && !admits(round.allowed[0], None));
        assert!(admits(round.allowed[1], None) && !admits(round.allowed[1], Some(b"seed")));
        for bad in [
            OUTCOME.replace("v:4:2:2", "s:4:2:2"),
            OUTCOME.replace("a:6:2:2", "a:7:2:2"),
            OUTCOME.replace("outcome\t0", "outcome\t1"),
        ] {
            assert!(replay(&format!("{SUBMIT}{bad}"), PLAN, &[[4, 6]], true).is_err());
        }
    }

    #[test]
    fn distinguishes_definite_rejection_unknown_and_torn_tail() {
        let body = format!("{SUBMIT}outcome\t0\t8\tu\tn\tu\tu\tn\tn\tu\n");
        let round = replay(&body, PLAN, &[[4, 6]], true).unwrap().pop().unwrap();
        assert_eq!((round.counts.unknown, round.counts.not_applied), (4, 3));
        assert!(
            admits(round.allowed[0], None)
                && admits(round.allowed[0], Some(b"seed"))
                && admits(round.allowed[0], Some(b"updated"))
        );
        assert!(admits(round.allowed[1], None) && !admits(round.allowed[1], Some(b"seed")));
        assert!(replay(SUBMIT, PLAN, &[[4, 6]], true).is_err());
        assert!(replay(SUBMIT, PLAN, &[[4, 6]], false).unwrap().is_empty());
    }
}
