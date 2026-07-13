use stateright::{Checker, Model, Property};

const MAX_NODES: usize = 4;

type ConfState = [bool; MAX_NODES];

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
struct MembershipCommitModel {
    max_steps: u8,
    drop_committed_entry_on_snapshot: bool,
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
struct ModelState {
    term: u8,
    leaders: [bool; MAX_NODES],
    committed_conf: ConfState,
    pending_conf: Option<ConfState>,
    committed_index: u8,
    applied_index: [u8; MAX_NODES],
    local_conf: [ConfState; MAX_NODES],
    faults_stopped: bool,
    steps: u8,
}

#[derive(Debug, Clone, Eq, PartialEq)]
enum Action {
    Elect(u8),
    ProposeAdd(u8),
    ProposeRemove(u8),
    CommitConf,
    CommitEntry,
    SnapshotInstall(u8),
    StopFaults,
}

impl MembershipCommitModel {
    fn checked() -> Self {
        Self {
            max_steps: 5,
            drop_committed_entry_on_snapshot: false,
        }
    }

    fn faulty_drops_committed_entry() -> Self {
        Self {
            max_steps: 5,
            drop_committed_entry_on_snapshot: true,
        }
    }
}

impl ModelState {
    fn initial() -> Self {
        let committed_conf = [true, true, true, false];
        let mut local_conf = [[false; MAX_NODES]; MAX_NODES];
        let mut applied_index = [0; MAX_NODES];
        for node in 0..MAX_NODES {
            if committed_conf[node] {
                local_conf[node] = committed_conf;
                applied_index[node] = 0;
            }
        }
        Self {
            term: 1,
            leaders: [true, false, false, false],
            committed_conf,
            pending_conf: None,
            committed_index: 0,
            applied_index,
            local_conf,
            faults_stopped: false,
            steps: 0,
        }
    }

    fn leader_count(&self) -> usize {
        self.leaders.iter().filter(|leader| **leader).count()
    }

    fn active_voter_count(&self) -> usize {
        self.committed_conf.iter().filter(|voter| **voter).count()
    }

    fn has_leader(&self) -> bool {
        self.leader_count() == 1
    }

    fn sync_committed_voters(&mut self) {
        for node in 0..MAX_NODES {
            if self.committed_conf[node] {
                self.local_conf[node] = self.committed_conf;
                self.applied_index[node] = self.committed_index;
            } else {
                self.leaders[node] = false;
            }
        }
        if self.leader_count() == 0 {
            if let Some(node) = first_voter(self.committed_conf) {
                self.leaders[node] = true;
            }
        }
    }

    fn converged(&self) -> bool {
        (0..MAX_NODES).all(|node| {
            !self.committed_conf[node]
                || (self.local_conf[node] == self.committed_conf
                    && self.applied_index[node] == self.committed_index)
        })
    }
}

impl Model for MembershipCommitModel {
    type State = ModelState;
    type Action = Action;

    fn init_states(&self) -> Vec<Self::State> {
        vec![ModelState::initial()]
    }

    fn actions(&self, state: &Self::State, actions: &mut Vec<Self::Action>) {
        if state.faults_stopped {
            return;
        }

        actions.push(Action::StopFaults);
        if state.steps >= self.max_steps {
            return;
        }

        for node in 0..MAX_NODES {
            let node = node as u8;
            if state.committed_conf[node as usize] {
                actions.push(Action::Elect(node));
                actions.push(Action::SnapshotInstall(node));
            }
        }

        if state.has_leader() {
            actions.push(Action::CommitEntry);
        }

        if state.pending_conf.is_some() {
            actions.push(Action::CommitConf);
            return;
        }

        for node in 0..MAX_NODES {
            let node = node as u8;
            if state.committed_conf[node as usize] {
                if state.active_voter_count() > 1 {
                    actions.push(Action::ProposeRemove(node));
                }
            } else {
                actions.push(Action::ProposeAdd(node));
            }
        }
    }

    fn next_state(&self, last_state: &Self::State, action: Self::Action) -> Option<Self::State> {
        let mut state = last_state.clone();
        match action {
            Action::Elect(node) => {
                if !state.committed_conf[node as usize] {
                    return None;
                }
                state.term = state.term.saturating_add(1);
                state.leaders = [false; MAX_NODES];
                state.leaders[node as usize] = true;
                state.steps = state.steps.saturating_add(1);
            }
            Action::ProposeAdd(node) => {
                if state.pending_conf.is_some() || state.committed_conf[node as usize] {
                    return None;
                }
                let mut next_conf = state.committed_conf;
                next_conf[node as usize] = true;
                state.pending_conf = Some(next_conf);
                state.steps = state.steps.saturating_add(1);
            }
            Action::ProposeRemove(node) => {
                if state.pending_conf.is_some()
                    || !state.committed_conf[node as usize]
                    || state.active_voter_count() <= 1
                {
                    return None;
                }
                let mut next_conf = state.committed_conf;
                next_conf[node as usize] = false;
                state.pending_conf = Some(next_conf);
                state.steps = state.steps.saturating_add(1);
            }
            Action::CommitConf => {
                let next_conf = state.pending_conf?;
                state.committed_conf = next_conf;
                state.pending_conf = None;
                state.sync_committed_voters();
                state.steps = state.steps.saturating_add(1);
            }
            Action::CommitEntry => {
                if !state.has_leader() {
                    return None;
                }
                state.committed_index = state.committed_index.saturating_add(1);
                state.sync_committed_voters();
                state.steps = state.steps.saturating_add(1);
            }
            Action::SnapshotInstall(node) => {
                if !state.committed_conf[node as usize] {
                    return None;
                }
                state.local_conf[node as usize] = state.committed_conf;
                state.applied_index[node as usize] =
                    if self.drop_committed_entry_on_snapshot && state.committed_index > 0 {
                        state.committed_index - 1
                    } else {
                        state.committed_index
                    };
                state.steps = state.steps.saturating_add(1);
            }
            Action::StopFaults => {
                state.faults_stopped = true;
                state.pending_conf = None;
                state.sync_committed_voters();
            }
        }
        Some(state)
    }

    fn properties(&self) -> Vec<Property<Self>> {
        vec![
            Property::<Self>::always("single leader per term", |_, state| {
                state.leader_count() <= 1
                    && (0..MAX_NODES)
                        .filter(|node| state.leaders[*node])
                        .all(|node| state.committed_conf[node])
            }),
            Property::<Self>::always("no committed entry lost", |_, state| {
                (0..MAX_NODES).all(|node| {
                    !state.committed_conf[node]
                        || state.applied_index[node] == state.committed_index
                })
            }),
            Property::<Self>::always("membership equals committed ConfState", |_, state| {
                (0..MAX_NODES).all(|node| {
                    !state.committed_conf[node] || state.local_conf[node] == state.committed_conf
                })
            }),
            Property::<Self>::eventually("converges after faults stop", |_, state| {
                state.faults_stopped && state.converged()
            }),
        ]
    }
}

#[test]
fn bounded_model_check_membership_and_commit_invariants_hold_for_up_to_4_nodes() {
    let checker = MembershipCommitModel::checked()
        .checker()
        .threads(1)
        .spawn_bfs()
        .join();

    assert!(
        checker.unique_state_count() > MAX_NODES,
        "model must explore a non-trivial bounded state space"
    );
    checker.assert_properties();
}

#[test]
fn canary_model_allows_a_dropped_committed_entry() {
    let checker = MembershipCommitModel::faulty_drops_committed_entry()
        .checker()
        .threads(1)
        .spawn_bfs()
        .join();

    assert!(
        checker.discovery("no committed entry lost").is_some(),
        "faulty snapshot install must produce a counterexample"
    );
}

fn first_voter(conf: ConfState) -> Option<usize> {
    (0..MAX_NODES).find(|node| conf[*node])
}
