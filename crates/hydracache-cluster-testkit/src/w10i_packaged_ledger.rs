//! Independent replay of the two packaged load clients' submitted batches and receipts.
use std::io::{BufRead, Read};
use std::ops::Range;

const ABSENT: u8 = 1;
const SEED: u8 = 2;
const UPDATED: u8 = 4;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct Counts {
    pub confirmed: usize,
    pub rejected: usize,
    pub not_applied: usize,
    pub unknown: usize,
}

#[derive(Debug)]
pub struct Round {
    pub keys: [String; 2],
    pub allowed: [u8; 2],
    pub counts: Counts,
    pub start: u64,
    pub end: u64,
}

impl Round {
    pub fn complete(&self) -> bool {
        self.counts.confirmed == 6 && self.counts.rejected == 1
    }

    pub fn in_window(&self, window: &Range<u64>) -> bool {
        self.complete() && window.contains(&self.start) && window.contains(&self.end)
    }
}

pub fn admits(allowed: u8, value: Option<&[u8]>) -> bool {
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

/// The short diagnostic and both long-run campaigns use the same 200 ms pacing.
/// Reserve ten extra seconds of slots for the final in-flight batch and shutdown.
pub fn planned_rounds(seconds: u64) -> Option<usize> {
    if !(1..=86_400).contains(&seconds) {
        return None;
    }
    usize::try_from((seconds + 10) * 5).ok()
}

const MAX_ROW_BYTES: u64 = 2_048;

/// One cursor over a journal and its immutable key plan. A live reader can stop
/// at an in-flight tail. A sealed journal must contain a result for every pair.
pub struct Replay<J, P> {
    journal: J,
    plan: P,
    sealed: bool,
    limit: usize,
    sequence: usize,
    previous_end: u64,
    key_prefix: String,
    last_key: Option<u64>,
    finished: bool,
}

impl<J: BufRead, P: BufRead> Replay<J, P> {
    pub fn new(journal: J, plan: P, sealed: bool, limit: usize, sdk: &str) -> Self {
        Self {
            journal,
            plan,
            sealed,
            limit,
            sequence: 0,
            previous_end: 0,
            finished: false,
            key_prefix: format!("w10i-load/{sdk}/")
                .bytes()
                .map(|byte| format!("{byte:02x}"))
                .collect(),
            last_key: None,
        }
    }

    fn next_round(&mut self) -> Result<Option<Round>, &'static str> {
        let Some(submit) = row(&mut self.journal, self.sealed)? else {
            return Ok(None);
        };
        let Some(outcome) = row(&mut self.journal, self.sealed)? else {
            return if self.sealed {
                Err("submission without an outcome")
            } else {
                Ok(None)
            };
        };
        if self.sequence >= self.limit {
            return Err("journal exceeded its planned round count");
        }
        let plan = row(&mut self.plan, true)?.ok_or("missing planned key pair")?;
        let round = parse_round(&submit, &outcome, &plan, self.sequence, self.previous_end)?;
        for key in &round.keys {
            let index =
                key_index(key, &self.key_prefix).ok_or("invalid SDK key namespace or index")?;
            if self.last_key.is_some_and(|previous| previous >= index) {
                return Err("SDK key plan reused or reordered a key");
            }
            self.last_key = Some(index);
        }
        self.sequence += 1;
        self.previous_end = round.end;
        Ok(Some(round))
    }
}

impl<J: BufRead, P: BufRead> Iterator for Replay<J, P> {
    type Item = Result<Round, &'static str>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.finished {
            return None;
        }
        match self.next_round() {
            Ok(Some(round)) => Some(Ok(round)),
            result => {
                self.finished = true;
                result.transpose()
            }
        }
    }
}

fn row(reader: &mut impl BufRead, sealed: bool) -> Result<Option<String>, &'static str> {
    let mut line = String::new();
    let bytes = reader
        .take(MAX_ROW_BYTES + 1)
        .read_line(&mut line)
        .map_err(|_| "cannot read row")?;
    if bytes as u64 > MAX_ROW_BYTES {
        return Err("ledger or plan row too large");
    }
    if bytes == 0 || (!sealed && !line.ends_with('\n')) {
        return Ok(None);
    }
    line.pop()
        .filter(|&end| end == '\n')
        .ok_or("incomplete final line")?;
    Ok(Some(line))
}

fn key_index(key: &str, prefix: &str) -> Option<u64> {
    let index = key.strip_prefix(prefix)?.as_bytes();
    if index.is_empty() || (index.len() > 2 && index.starts_with(b"30")) {
        return None;
    }
    index.chunks_exact(2).try_fold(0_u64, |value, pair| {
        let [b'3', digit @ b'0'..=b'9'] = *pair else {
            return None;
        };
        value.checked_mul(10)?.checked_add(u64::from(digit - b'0'))
    })
}

fn parse_round(
    submit: &str,
    outcome: &str,
    plan: &str,
    seq: usize,
    previous_end: u64,
) -> Result<Round, &'static str> {
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
    let columns: Vec<_> = plan.split('\t').collect();
    let [planned_a, planned_b, part_a, part_b] = columns.as_slice() else {
        return Err("invalid plan row");
    };
    let partition =
        [part_a, part_b].map(|part| part.parse::<u32>().ok().filter(|&part| part < 271));
    if sent_seq.parse::<usize>().ok() != Some(seq)
        || sent_seq != result_seq
        || start < previous_end
        || end < start
        || (*planned_a, *planned_b) != (*a, *b)
        || [a, b].iter().any(|key| {
            key.is_empty()
                || key.len() % 2 != 0
                || !key
                    .bytes()
                    .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        })
        || partition.contains(&None)
        || a == b
        || items.len() != 7
    {
        return Err("ledger order, timing, plan or item count mismatch");
    }
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
        if part.parse::<u32>().ok() != partition[key]
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
    Ok(Round {
        keys: [a.to_string(), b.to_string()],
        allowed: keys,
        start,
        end,
        counts,
    })
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct Windows {
    pub outage: Range<u64>,
    pub held: Range<u64>,
    pub returned: Range<u64>,
}

#[derive(Debug, Default, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct Progress {
    pub counts: Counts,
    pub rounds: usize,
    pub keys: usize,
    pub offline_rounds: usize,
    pub held_rounds: usize,
    pub post_return_rounds: usize,
}

#[derive(Default)]
pub struct Coverage {
    first_start: Option<u64>,
    last_end: u64,
    progress: Progress,
}

impl Coverage {
    pub fn observe(&mut self, round: &Round, windows: &Windows) -> Result<(), &'static str> {
        if self.first_start.is_some() && round.start.saturating_sub(self.last_end) > 1_000 {
            return Err("packaged workload has an inter-round gap");
        }
        self.first_start.get_or_insert(round.start);
        self.last_end = round.end;
        let progress = &mut self.progress;
        progress.rounds += 1;
        progress.keys += round.keys.len();
        progress.counts.confirmed += round.counts.confirmed;
        progress.counts.rejected += round.counts.rejected;
        progress.counts.not_applied += round.counts.not_applied;
        progress.counts.unknown += round.counts.unknown;
        progress.offline_rounds += usize::from(round.in_window(&windows.outage));
        progress.held_rounds += usize::from(round.in_window(&windows.held));
        progress.post_return_rounds += usize::from(round.in_window(&windows.returned));
        Ok(())
    }

    pub fn finish(self, windows: &Windows) -> Result<Progress, &'static str> {
        if self
            .first_start
            .is_none_or(|first| first >= windows.outage.start || first > 10_000)
            || self.last_end.saturating_add(1_000) < windows.returned.end
            || self.last_end > windows.returned.end.saturating_add(10_000)
            || self.progress.offline_rounds == 0
            || self.progress.held_rounds == 0
            || self.progress.post_return_rounds == 0
        {
            return Err("packaged workload lacks traffic at a required cut or duration boundary");
        }
        Ok(self.progress)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PLAN: &str = "773130692d6c6f61642f727573742f30\t773130692d6c6f61642f727573742f31\t4\t6\n";
    const SUBMIT: &str =
        "submit\t0\t5\t773130692d6c6f61642f727573742f30\t773130692d6c6f61642f727573742f31\n";
    const OUTCOME: &str =
        "outcome\t0\t8\ta:4:2:1\tr:4:2:1\ta:4:2:2\tv:4:2:2\ta:6:2:1\ta:6:2:2\t0:6:2:2\n";

    fn replay(body: &str, sealed: bool) -> Result<Vec<Round>, &'static str> {
        Replay::new(body.as_bytes(), PLAN.as_bytes(), sealed, 1, "rust").collect()
    }

    #[test]
    fn plans_both_long_profiles_and_fences_limits_and_partial_rows() {
        assert_eq!(planned_rounds(60), Some(350));
        assert_eq!(planned_rounds(21_600), Some(108_050));
        assert_eq!(planned_rounds(86_400), Some(432_050));
        assert_eq!(planned_rounds(0), None);
        assert_eq!(planned_rounds(u64::MAX), None);
        assert!(replay(&format!("{SUBMIT}{}", "x".repeat(2049)), false).is_err());
        assert!(replay(&format!("{SUBMIT}{}", OUTCOME.trim_end()), true).is_err());
        assert!(replay(&format!("{SUBMIT}{}", OUTCOME.trim_end()), false)
            .unwrap()
            .is_empty());
        let body = format!("{SUBMIT}{OUTCOME}{SUBMIT}{OUTCOME}");
        let mut rounds = Replay::new(body.as_bytes(), PLAN.as_bytes(), true, 1, "rust");
        assert!(rounds.next().unwrap().is_ok());
        assert!(rounds.next().unwrap().is_err());
        assert!(rounds.next().is_none());
    }

    #[test]
    fn unique_sdk_keys_are_proven_without_retaining_the_plan() {
        let wrong_sdk = format!("{SUBMIT}{OUTCOME}");
        assert!(
            Replay::new(wrong_sdk.as_bytes(), PLAN.as_bytes(), true, 2, "java")
                .next()
                .unwrap()
                .is_err()
        );
        let repeated = format!(
            "{SUBMIT}{OUTCOME}{}{}",
            SUBMIT.replace("submit\t0\t5", "submit\t1\t9"),
            OUTCOME.replace("outcome\t0\t8", "outcome\t1\t10")
        );
        let plan = PLAN.repeat(2);
        let mut rounds = Replay::new(repeated.as_bytes(), plan.as_bytes(), true, 2, "rust");
        assert!(rounds.next().unwrap().is_ok());
        assert_eq!(
            rounds.next().unwrap().unwrap_err(),
            "SDK key plan reused or reordered a key"
        );
    }

    #[test]
    fn coverage_requires_both_duration_boundaries_and_every_fault_window() {
        let windows = Windows {
            outage: 1_000..2_000,
            held: 2_000..3_000,
            returned: 3_000..4_000,
        };
        let mut round = replay(&format!("{SUBMIT}{OUTCOME}"), true)
            .unwrap()
            .remove(0);
        let mut coverage = Coverage::default();
        coverage.observe(&round, &windows).unwrap();
        for start in [1_000, 2_000, 3_000, 3_900] {
            round.start = start;
            round.end = start + 50;
            coverage.observe(&round, &windows).unwrap();
        }
        let result = coverage.finish(&windows).unwrap();
        assert_eq!((result.rounds, result.keys), (5, 10));
        assert_eq!(
            (
                result.offline_rounds,
                result.held_rounds,
                result.post_return_rounds
            ),
            (1, 1, 2)
        );
        let mut missing = Coverage::default();
        round.start = 1_000;
        round.end = 1_050;
        missing.observe(&round, &windows).unwrap();
        assert!(missing.finish(&windows).is_err());
        let mut gapped = Coverage::default();
        gapped.observe(&round, &windows).unwrap();
        round.start = 2_051;
        round.end = 2_100;
        assert!(gapped.observe(&round, &windows).is_err());
    }

    #[test]
    fn replay_fences_confirmed_values_and_deletions() {
        let body = format!("{SUBMIT}{OUTCOME}");
        let round = replay(&body, true).unwrap().pop().unwrap();
        assert!(round.in_window(&(4..9)) && !round.in_window(&(5..8)));
        assert!(admits(round.allowed[0], Some(b"updated")) && !admits(round.allowed[0], None));
        assert!(admits(round.allowed[1], None) && !admits(round.allowed[1], Some(b"seed")));
        for bad in [
            OUTCOME.replace("v:4:2:2", "s:4:2:2"),
            OUTCOME.replace("a:6:2:2", "a:7:2:2"),
            OUTCOME.replace("outcome\t0", "outcome\t1"),
        ] {
            assert!(replay(&format!("{SUBMIT}{bad}"), true).is_err());
        }
    }

    #[test]
    fn distinguishes_definite_rejection_unknown_and_torn_tail() {
        let body = format!("{SUBMIT}outcome\t0\t8\tu\tn\tu\tu\tn\tn\tu\n");
        let round = replay(&body, true).unwrap().pop().unwrap();
        assert_eq!((round.counts.unknown, round.counts.not_applied), (4, 3));
        assert!(
            admits(round.allowed[0], None)
                && admits(round.allowed[0], Some(b"seed"))
                && admits(round.allowed[0], Some(b"updated"))
        );
        assert!(admits(round.allowed[1], None) && !admits(round.allowed[1], Some(b"seed")));
        assert!(replay(SUBMIT, true).is_err());
        assert!(replay(SUBMIT, false).unwrap().is_empty());
    }
}
