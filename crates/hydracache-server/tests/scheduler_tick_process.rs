mod support;

#[cfg(target_os = "linux")]
use std::collections::BTreeMap;
use std::collections::BTreeSet;
#[cfg(target_os = "linux")]
use std::fs;
#[cfg(target_os = "linux")]
use std::path::{Path, PathBuf};
#[cfg(target_os = "linux")]
use std::time::{Duration, Instant};

#[cfg(target_os = "linux")]
use serde::Serialize;
#[cfg(target_os = "linux")]
use support::daemon_cluster::DaemonCluster;
use support::daemon_cluster::{skip_unless_daemon_process_e2e, DaemonStatus, TestResult};
#[cfg(target_os = "linux")]
use support::membership_history::MembershipHistoryRecorder;
use support::membership_history::MembershipObservation;

#[cfg(target_os = "linux")]
#[derive(Debug, Clone, Serialize)]
struct AdminLeaderSample {
    stage: String,
    node_index: usize,
    term: u64,
    leader: Option<String>,
    members: u32,
    voters: u32,
    quorum_ok: bool,
    draining: bool,
}

#[cfg(target_os = "linux")]
#[derive(Debug, Default)]
struct SchedulerTickEvidence {
    test_name: &'static str,
    stages: Vec<String>,
    admin_samples: Vec<AdminLeaderSample>,
    overview_samples: Vec<MembershipObservation>,
    authoritative_membership: MembershipHistoryRecorder,
}

#[cfg(target_os = "linux")]
impl SchedulerTickEvidence {
    fn new(test_name: &'static str) -> Self {
        Self {
            test_name,
            ..Self::default()
        }
    }

    fn capture_all(&mut self, cluster: &mut DaemonCluster, stage: impl Into<String>) -> TestResult {
        let stage = stage.into();
        self.stages.push(stage.clone());
        let indices = cluster.running_indices();
        let mut statuses = Vec::new();
        for index in indices {
            if let Ok(status) = cluster.admin_status(index) {
                self.record_admin(&stage, index, status.clone());
                statuses.push((index, status));
            }
        }
        if statuses.is_empty() {
            return Err(format!("{stage} captured no public /admin/status responses").into());
        }
        let leaders = statuses
            .iter()
            .filter_map(|(_, status)| status.leader.clone())
            .collect::<BTreeSet<_>>();
        if let [leader] = leaders.iter().collect::<Vec<_>>().as_slice() {
            if let Some(index) = cluster
                .node_ids()
                .iter()
                .position(|node_id| node_id == *leader)
            {
                let overview = cluster.cluster_overview(index)?;
                self.record_authoritative_overview(&overview);
            }
        }
        Ok(())
    }

    fn capture_node(
        &mut self,
        cluster: &DaemonCluster,
        node_index: usize,
        stage: impl Into<String>,
    ) -> TestResult<(DaemonStatus, MembershipObservation)> {
        let stage = stage.into();
        self.stages.push(stage.clone());
        let status = cluster.admin_status(node_index)?;
        self.record_admin(&stage, node_index, status.clone());
        let overview = cluster.cluster_overview(node_index)?;
        let membership = MembershipObservation::from_cluster_overview(&overview);
        self.overview_samples.push(membership.clone());
        Ok((status, membership))
    }

    fn record_authoritative_overview(&mut self, overview: &serde_json::Value) {
        let observation = MembershipObservation::from_cluster_overview(overview);
        self.record_authoritative_membership(observation);
    }

    fn record_authoritative_membership(&mut self, observation: MembershipObservation) {
        self.overview_samples.push(observation.clone());
        self.authoritative_membership.record(observation);
    }

    fn record_admin(&mut self, stage: &str, node_index: usize, status: DaemonStatus) {
        self.admin_samples.push(AdminLeaderSample {
            stage: stage.to_owned(),
            node_index,
            term: status.term,
            leader: status.leader,
            members: status.members,
            voters: status.voters,
            quorum_ok: status.quorum_ok,
            draining: status.draining,
        });
    }

    fn assert_single_leader_per_term(&self) {
        let mut leaders_by_term = BTreeMap::<u64, BTreeSet<String>>::new();
        for sample in &self.admin_samples {
            if let Some(leader) = &sample.leader {
                leaders_by_term
                    .entry(sample.term)
                    .or_default()
                    .insert(leader.clone());
            }
        }
        for (term, leaders) in leaders_by_term {
            assert!(
                leaders.len() <= 1,
                "public admin history reported two leaders in term {term}: {leaders:?}; samples={:?}",
                self.admin_samples
            );
        }
    }

    fn assert_membership_history(&self) {
        let report = self.authoritative_membership.check();
        assert!(
            report.is_ok(),
            "scheduler/tick membership history violated invariants: {:?}; observations={:?}",
            report.violations,
            self.authoritative_membership.observations()
        );
    }

    fn persist(&self, root: &Path) -> TestResult<PathBuf> {
        #[derive(Serialize)]
        struct Artifact<'a> {
            schema_version: u32,
            release: &'static str,
            test_name: &'a str,
            stages: &'a [String],
            admin_samples: &'a [AdminLeaderSample],
            overview_samples: &'a [MembershipObservation],
            authoritative_membership: &'a [MembershipObservation],
        }
        let path = root.join(format!("{}-history.json", self.test_name));
        let artifact = Artifact {
            schema_version: 1,
            release: "0.66.0",
            test_name: self.test_name,
            stages: &self.stages,
            admin_samples: &self.admin_samples,
            overview_samples: &self.overview_samples,
            authoritative_membership: self.authoritative_membership.observations(),
        };
        fs::write(&path, serde_json::to_vec_pretty(&artifact)?)?;
        Ok(path)
    }
}

#[test]
fn process_pause_and_uneven_ticks_never_create_two_leaders_per_term() -> TestResult {
    if !skip_unless_daemon_process_e2e(
        "process_pause_and_uneven_ticks_never_create_two_leaders_per_term",
    ) {
        return Ok(());
    }
    run_pause_and_uneven_ticks()
}

#[test]
fn resumed_demoted_process_never_reports_authoritative_membership() -> TestResult {
    if !skip_unless_daemon_process_e2e(
        "resumed_demoted_process_never_reports_authoritative_membership",
    ) {
        return Ok(());
    }
    run_resumed_demoted_process()
}

#[cfg(not(target_os = "linux"))]
fn run_pause_and_uneven_ticks() -> TestResult {
    eprintln!(
        "skipping real process scheduler/tick proof: SIGSTOP/SIGCONT is supported only on Linux"
    );
    Ok(())
}

#[cfg(not(target_os = "linux"))]
fn run_resumed_demoted_process() -> TestResult {
    eprintln!("skipping real resumed-demotion proof: SIGSTOP/SIGCONT is supported only on Linux");
    Ok(())
}

#[cfg(target_os = "linux")]
fn run_pause_and_uneven_ticks() -> TestResult {
    let test_name = "process_pause_and_uneven_ticks_never_create_two_leaders_per_term";
    let mut cluster = DaemonCluster::start_bootstrap(3, "scheduler-uneven-ticks")?;
    let mut evidence = SchedulerTickEvidence::new(test_name);
    let initial = cluster.wait_for_shape(3, 3)?;
    let old_leader = initial[0]
        .leader
        .clone()
        .ok_or("scheduler process test observed no initial leader")?;
    let old_leader_index = node_index(&cluster, &old_leader)?;
    evidence.capture_all(&mut cluster, "initial")?;

    cluster.suspend(old_leader_index)?;
    std::thread::sleep(Duration::from_millis(75));
    cluster.wait_for_leader_not(&old_leader, 3, 3)?;
    evidence.capture_all(&mut cluster, "paused-leader-majority-elected")?;
    for (index, delay) in [17_u64, 83, 211].into_iter().enumerate() {
        std::thread::sleep(Duration::from_millis(delay));
        evidence.capture_all(&mut cluster, format!("paused-uneven-window-{index}"))?;
    }

    cluster.resume(old_leader_index)?;
    for index in 0..6 {
        std::thread::sleep(Duration::from_millis(25));
        if evidence
            .capture_node(
                &cluster,
                old_leader_index,
                format!("resumed-former-leader-{index}"),
            )
            .is_err()
        {
            continue;
        }
    }
    let converged = cluster.wait_for_shape(3, 3)?;
    evidence.capture_all(&mut cluster, "resumed-converged")?;

    let current_leader = converged[0]
        .leader
        .as_deref()
        .ok_or("scheduler process test lost leader after resume")?;
    let uneven_follower = cluster
        .node_ids()
        .iter()
        .position(|node_id| node_id != current_leader)
        .ok_or("scheduler process test found no follower for second pause")?;
    cluster.suspend(uneven_follower)?;
    for (index, delay) in [31_u64, 127, 263].into_iter().enumerate() {
        std::thread::sleep(Duration::from_millis(delay));
        evidence.capture_all(&mut cluster, format!("follower-uneven-window-{index}"))?;
    }
    cluster.resume(uneven_follower)?;
    cluster.wait_for_shape(3, 3)?;
    evidence.capture_all(&mut cluster, "all-processes-converged")?;

    evidence.assert_single_leader_per_term();
    evidence.assert_membership_history();
    let artifact = evidence.persist(cluster.root())?;
    assert!(artifact.is_file());
    assert_daemon_logs_preserved(&mut cluster);
    Ok(())
}

#[cfg(target_os = "linux")]
fn run_resumed_demoted_process() -> TestResult {
    let test_name = "resumed_demoted_process_never_reports_authoritative_membership";
    let mut cluster = DaemonCluster::start_bootstrap(3, "scheduler-resumed-demotion")?;
    let mut evidence = SchedulerTickEvidence::new(test_name);
    let initial = cluster.wait_for_shape(3, 3)?;
    let old_leader = initial[0]
        .leader
        .clone()
        .ok_or("resumed-demotion test observed no initial leader")?;
    let old_leader_index = node_index(&cluster, &old_leader)?;
    evidence.capture_all(&mut cluster, "before-suspend")?;

    cluster.suspend(old_leader_index)?;
    let replacement = cluster.wait_for_leader_not(&old_leader, 3, 3)?;
    let replacement_leader = replacement[0]
        .leader
        .clone()
        .ok_or("resumed-demotion test observed no replacement leader")?;
    let replacement_term = replacement[0].term;
    let replacement_index = node_index(&cluster, &replacement_leader)?;
    evidence.capture_all(&mut cluster, "replacement-authoritative")?;

    let node_ids = cluster.node_ids();
    let drain_index = node_ids
        .iter()
        .enumerate()
        .find(|(index, node_id)| {
            *index != old_leader_index && node_id.as_str() != replacement_leader
        })
        .map(|(index, _)| index)
        .ok_or("resumed-demotion test found no live follower to drain while leader was paused")?;
    let drained_node_id = node_ids[drain_index].clone();
    let accepted = cluster.drain(drain_index)?;
    assert_eq!(accepted["outcome"], "accepted");
    let (committed_while_paused, committed_membership) = cluster.wait_for(
        "committed membership projected while former leader is suspended".to_owned(),
        |cluster| {
            let status = cluster.admin_status(replacement_index).ok()?;
            if status.members != 2 || status.voters != 2 {
                return None;
            }
            let membership = MembershipObservation::from_cluster_overview(
                &cluster.cluster_overview(replacement_index).ok()?,
            );
            // Committing the drain leaves exactly the replacement and the
            // suspended former leader as voters. The live process therefore
            // loses quorum immediately after the commit and cannot advertise
            // a leader or non-zero authoritative epoch until SIGCONT. Preserve
            // this exact projection as diagnostics only; quorum/leader
            // authority is required again after resume below.
            (membership.members.len() == 2 && !membership.members.contains(&drained_node_id))
                .then_some((status, membership))
        },
    )?;
    evidence.record_admin(
        "membership-committed-while-former-leader-paused",
        replacement_index,
        committed_while_paused,
    );
    assert_eq!(committed_membership.members.len(), 2);
    assert!(
        !committed_membership.members.contains(&drained_node_id),
        "committed membership still contained drained node {drained_node_id}: {committed_membership:?}"
    );
    evidence.overview_samples.push(committed_membership);

    cluster.resume(old_leader_index)?;
    let deadline = Instant::now() + Duration::from_secs(5);
    let mut aligned_authoritative_sample = None;
    while Instant::now() < deadline {
        let Some((peer_index, peer_status)) = authoritative_peer_view(&cluster, old_leader_index)
        else {
            std::thread::sleep(Duration::from_millis(20));
            continue;
        };
        evidence.record_admin(
            "live-majority-peer-public-view",
            peer_index,
            peer_status.clone(),
        );
        let Ok((resumed_status, resumed_membership)) = evidence.capture_node(
            &cluster,
            old_leader_index,
            "resumed-former-leader-public-view",
        ) else {
            std::thread::sleep(Duration::from_millis(20));
            continue;
        };
        let peer_membership =
            MembershipObservation::from_cluster_overview(&cluster.cluster_overview(peer_index)?);
        // `/admin/status` can become reachable a few scheduler ticks before the
        // public overview has materialized its committed epoch. Epoch zero is a
        // bootstrap/uninitialized view, not an authoritative membership
        // regression, so wait for the majority peer's committed shape before
        // adding it to the monotonic history.
        if !public_membership_is_authoritative(&peer_status, &peer_membership, 2, 2) {
            std::thread::sleep(Duration::from_millis(20));
            continue;
        }
        evidence.record_authoritative_membership(peer_membership.clone());

        // The two endpoints are separate snapshots. After SIGCONT the former
        // leader can regain a leader/quorum view before it applies the peer's
        // committed membership epoch. Treat the sample as authoritative only
        // once /admin/status itself proves that the membership transition has
        // been applied; the bounded deadline below still fails closed if the
        // node remains stale.
        if public_membership_is_authoritative(&resumed_status, &resumed_membership, 2, 2) {
            assert_eq!(
                (resumed_status.term, resumed_status.leader.as_deref()),
                (peer_status.term, peer_status.leader.as_deref()),
                "resumed former leader reported an authoritative stale leader/term"
            );
            assert_eq!(
                resumed_membership, peer_membership,
                "resumed former leader reported an authoritative stale /cluster/overview membership view"
            );
            evidence.record_authoritative_membership(resumed_membership);
            aligned_authoritative_sample = Some(resumed_status);
            break;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    let aligned = aligned_authoritative_sample.ok_or(
        "resumed former leader never exposed an authoritative view aligned with the live majority",
    )?;
    if aligned.leader.as_deref() == Some(old_leader.as_str()) {
        assert!(
            aligned.term > replacement_term,
            "former leader may be re-elected only in a later term after catching up: replacement_term={replacement_term} aligned={aligned:?}"
        );
    } else {
        assert_eq!(
            aligned.leader.as_deref(),
            Some(replacement_leader.as_str()),
            "resumed process exposed a leader other than the live replacement or a later-term re-election"
        );
    }

    cluster.wait_for_non_draining_shape("post-resume membership commit", 2, 2)?;
    let final_deadline = Instant::now() + Duration::from_secs(5);
    let (resumed_after_commit, resumed_membership) = loop {
        let sample = evidence.capture_node(
            &cluster,
            old_leader_index,
            "resumed-process-after-membership-commit",
        )?;
        if public_membership_is_authoritative(&sample.0, &sample.1, 2, 2) {
            break sample;
        }
        if Instant::now() >= final_deadline {
            return Err(
                "post-resume status and membership overview did not align before deadline".into(),
            );
        }
        std::thread::sleep(Duration::from_millis(20));
    };
    assert_eq!(resumed_after_commit.members, 2);
    assert_eq!(resumed_after_commit.voters, 2);
    evidence.record_authoritative_membership(resumed_membership);
    evidence.capture_all(&mut cluster, "post-resume-committed-membership")?;

    evidence.assert_single_leader_per_term();
    evidence.assert_membership_history();
    let artifact = evidence.persist(cluster.root())?;
    assert!(artifact.is_file());
    assert_daemon_logs_preserved(&mut cluster);
    Ok(())
}

fn public_membership_is_authoritative(
    status: &DaemonStatus,
    membership: &MembershipObservation,
    expected_members: u32,
    expected_voters: u32,
) -> bool {
    status.quorum_ok
        && status.leader.is_some()
        && status.epoch != 0
        && status.term != 0
        && status.members == expected_members
        && status.voters == expected_voters
        && membership.epoch == status.epoch
        && membership.term == status.term
        && membership.leader == status.leader
        && membership.members.len()
            == usize::try_from(expected_members).expect("member count fits usize")
}

#[test]
fn public_membership_requires_an_aligned_authoritative_status_and_overview() {
    let members = ["node-a".to_owned(), "node-b".to_owned()]
        .into_iter()
        .collect::<BTreeSet<_>>();
    let status = DaemonStatus {
        leader: Some("node-a".to_owned()),
        term: 11,
        epoch: 4,
        members: 2,
        voters: 2,
        quorum_ok: true,
        draining: false,
    };
    let aligned = MembershipObservation {
        epoch: 4,
        term: 11,
        leader: Some("node-a".to_owned()),
        members: members.clone(),
    };
    assert!(public_membership_is_authoritative(&status, &aligned, 2, 2));

    let bootstrap = MembershipObservation {
        epoch: 0,
        term: 0,
        leader: None,
        members: members.clone(),
    };
    assert!(!public_membership_is_authoritative(
        &status, &bootstrap, 2, 2
    ));

    let mut stale_leader = aligned.clone();
    stale_leader.leader = Some("node-b".to_owned());
    assert!(!public_membership_is_authoritative(
        &status,
        &stale_leader,
        2,
        2
    ));

    let mut non_quorate = status;
    non_quorate.quorum_ok = false;
    assert!(!public_membership_is_authoritative(
        &non_quorate,
        &aligned,
        2,
        2
    ));
}

#[cfg(target_os = "linux")]
fn authoritative_peer_view(
    cluster: &DaemonCluster,
    excluded_index: usize,
) -> Option<(usize, DaemonStatus)> {
    let peers = cluster
        .node_ids()
        .iter()
        .enumerate()
        .filter(|(index, _)| *index != excluded_index)
        .filter_map(|(index, _)| {
            cluster
                .admin_status(index)
                .ok()
                .map(|status| (index, status))
        })
        .filter(|(_, status)| status.quorum_ok && status.leader.is_some())
        .collect::<Vec<_>>();
    let first = peers.first()?.clone();
    peers
        .iter()
        .all(|(_, status)| {
            (status.term, status.leader.as_deref()) == (first.1.term, first.1.leader.as_deref())
        })
        .then_some(first)
}

#[cfg(target_os = "linux")]
fn node_index(cluster: &DaemonCluster, node_id: &str) -> TestResult<usize> {
    cluster
        .node_ids()
        .iter()
        .position(|candidate| candidate == node_id)
        .ok_or_else(|| format!("leader {node_id} did not belong to DaemonCluster").into())
}

#[cfg(target_os = "linux")]
fn assert_daemon_logs_preserved(cluster: &mut DaemonCluster) {
    let replay = cluster.replay_evidence(None);
    assert!(
        replay
            .stdout_logs
            .iter()
            .chain(replay.stderr_logs.iter())
            .all(|path| path.is_file()),
        "scheduler/tick evidence must preserve child logs: {replay:?}"
    );
}
