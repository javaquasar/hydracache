---------------------------- MODULE RaftElection ----------------------------
EXTENDS Naturals, FiniteSets, Sequences, TLC

CONSTANTS Nodes, Follower, Candidate, Leader, Nil, ClusterId,
          MaxTerm, MaxLogIndex, MaxMessages

ASSUME /\ Cardinality(Nodes) >= 3
       /\ MaxTerm >= 1
       /\ MaxLogIndex >= 1
       /\ MaxMessages >= 1

VARIABLES role, term, maxTermSeen, votedFor, preVotes, votes, available,
          logTerm, commitIndex, maxCommitSeen, appliedIndex,
          snapshotIndex, maxSnapshotSeen, snapshotCluster,
          activeMembers, membershipEpoch, messages, faultsStopped

vars == <<role, term, maxTermSeen, votedFor, preVotes, votes, available,
          logTerm, commitIndex, maxCommitSeen, appliedIndex,
          snapshotIndex, maxSnapshotSeen, snapshotCluster,
          activeMembers, membershipEpoch, messages, faultsStopped>>

Quorum == Cardinality(Nodes) \div 2 + 1
NodeSymmetry == Permutations(Nodes)
QuorumAvailable == Cardinality({n \in activeMembers : available[n]}) >= Quorum
HasLeader == \E n \in activeMembers : available[n] /\ role[n] = Leader
NoLeader == ~HasLeader

Message == [kind : {"PreVote", "Vote"}, source : Nodes, dest : Nodes,
            msgTerm : 0..MaxTerm]

RemoveAt(sequence, index) ==
    [position \in 1..(Len(sequence) - 1) |->
        IF position < index THEN sequence[position] ELSE sequence[position + 1]]

Prefix(sequence, length) ==
    IF length = 0 THEN <<>> ELSE SubSeq(sequence, 1, length)

MinValue(left, right) == IF left <= right THEN left ELSE right
MaxValue(left, right) == IF left >= right THEN left ELSE right

Init ==
    /\ role = [n \in Nodes |-> Follower]
    /\ term = [n \in Nodes |-> 0]
    /\ maxTermSeen = [n \in Nodes |-> 0]
    /\ votedFor = [n \in Nodes |-> Nil]
    /\ preVotes = [n \in Nodes |-> {}]
    /\ votes = [n \in Nodes |-> {}]
    /\ available = [n \in Nodes |-> TRUE]
    /\ logTerm = [n \in Nodes |-> <<>>]
    /\ commitIndex = [n \in Nodes |-> 0]
    /\ maxCommitSeen = [n \in Nodes |-> 0]
    /\ appliedIndex = [n \in Nodes |-> 0]
    /\ snapshotIndex = [n \in Nodes |-> 0]
    /\ maxSnapshotSeen = [n \in Nodes |-> 0]
    /\ snapshotCluster = [n \in Nodes |-> Nil]
    /\ activeMembers = Nodes
    /\ membershipEpoch = 0
    /\ messages = <<>>
    /\ faultsStopped = FALSE

StartPreVote(n) ==
    /\ n \in activeMembers
    /\ available[n]
    /\ role[n] = Follower
    /\ term[n] < MaxTerm
    /\ preVotes' = [preVotes EXCEPT ![n] = {n}]
    /\ UNCHANGED <<role, term, maxTermSeen, votedFor, votes, available,
                   logTerm, commitIndex, maxCommitSeen, appliedIndex,
                   snapshotIndex, maxSnapshotSeen, snapshotCluster,
                   activeMembers, membershipEpoch, messages, faultsStopped>>

SendPreVote(n, peer) ==
    /\ n /= peer
    /\ n \in activeMembers
    /\ peer \in activeMembers
    /\ available[n]
    /\ role[n] = Follower
    /\ term[n] < MaxTerm
    /\ preVotes[n] /= {}
    /\ Len(messages) < MaxMessages
    /\ messages' = Append(messages,
          [kind |-> "PreVote", source |-> n, dest |-> peer,
           msgTerm |-> term[n] + 1])
    /\ UNCHANGED <<role, term, maxTermSeen, votedFor, preVotes, votes,
                   available, logTerm, commitIndex, maxCommitSeen, appliedIndex,
                   snapshotIndex, maxSnapshotSeen, snapshotCluster,
                   activeMembers, membershipEpoch, faultsStopped>>

DeliverPreVote(index) ==
    LET message == messages[index] IN
    /\ message.kind = "PreVote"
    /\ available[message.source]
    /\ available[message.dest]
    /\ message.source \in activeMembers
    /\ message.dest \in activeMembers
    /\ message.msgTerm > term[message.dest]
    /\ preVotes' = [preVotes EXCEPT
          ![message.source] = @ \union {message.dest}]
    /\ messages' = RemoveAt(messages, index)
    /\ UNCHANGED <<role, term, maxTermSeen, votedFor, votes, available,
                   logTerm, commitIndex, maxCommitSeen, appliedIndex,
                   snapshotIndex, maxSnapshotSeen, snapshotCluster,
                   activeMembers, membershipEpoch, faultsStopped>>

StartElection(n) ==
    /\ n \in activeMembers
    /\ available[n]
    /\ Cardinality(preVotes[n]) >= Quorum
    /\ term[n] < MaxTerm
    /\ role' = [role EXCEPT ![n] = Candidate]
    /\ term' = [term EXCEPT ![n] = @ + 1]
    /\ maxTermSeen' = [maxTermSeen EXCEPT ![n] = term'[n]]
    /\ votedFor' = [votedFor EXCEPT ![n] = n]
    /\ preVotes' = [preVotes EXCEPT ![n] = {}]
    /\ votes' = [votes EXCEPT ![n] = {n}]
    /\ UNCHANGED <<available, logTerm, commitIndex, maxCommitSeen,
                   appliedIndex, snapshotIndex, maxSnapshotSeen,
                   snapshotCluster, activeMembers, membershipEpoch,
                   messages, faultsStopped>>

SendVote(n, peer) ==
    /\ n /= peer
    /\ n \in activeMembers
    /\ peer \in activeMembers
    /\ available[n]
    /\ role[n] = Candidate
    /\ Len(messages) < MaxMessages
    /\ messages' = Append(messages,
          [kind |-> "Vote", source |-> n, dest |-> peer,
           msgTerm |-> term[n]])
    /\ UNCHANGED <<role, term, maxTermSeen, votedFor, preVotes, votes,
                   available, logTerm, commitIndex, maxCommitSeen, appliedIndex,
                   snapshotIndex, maxSnapshotSeen, snapshotCluster,
                   activeMembers, membershipEpoch, faultsStopped>>

DeliverVote(index) ==
    LET message == messages[index] IN
    /\ message.kind = "Vote"
    /\ available[message.source]
    /\ available[message.dest]
    /\ message.source \in activeMembers
    /\ message.dest \in activeMembers
    /\ message.msgTerm >= term[message.dest]
    /\ \/ votedFor[message.dest] = Nil
       \/ votedFor[message.dest] = message.source
       \/ message.msgTerm > term[message.dest]
    /\ role' = [role EXCEPT ![message.dest] = Follower]
    /\ term' = [term EXCEPT ![message.dest] = message.msgTerm]
    /\ maxTermSeen' = [maxTermSeen EXCEPT ![message.dest] = message.msgTerm]
    /\ votedFor' = [votedFor EXCEPT ![message.dest] = message.source]
    /\ votes' = [votes EXCEPT
          ![message.source] = @ \union {message.dest}]
    /\ messages' = RemoveAt(messages, index)
    /\ UNCHANGED <<preVotes, available, logTerm, commitIndex,
                   maxCommitSeen, appliedIndex, snapshotIndex,
                   maxSnapshotSeen, snapshotCluster, activeMembers,
                   membershipEpoch, faultsStopped>>

BecomeLeader(n) ==
    /\ n \in activeMembers
    /\ available[n]
    /\ role[n] = Candidate
    /\ Cardinality(votes[n]) >= Quorum
    /\ \A peer \in activeMembers :
          term[peer] = term[n] => role[peer] /= Leader
    /\ role' = [role EXCEPT ![n] = Leader]
    /\ UNCHANGED <<term, maxTermSeen, votedFor, preVotes, votes, available,
                   logTerm, commitIndex, maxCommitSeen, appliedIndex,
                   snapshotIndex, maxSnapshotSeen, snapshotCluster,
                   activeMembers, membershipEpoch, messages, faultsStopped>>

AppendCommitted(leader) ==
    /\ leader \in activeMembers
    /\ available[leader]
    /\ role[leader] = Leader
    /\ QuorumAvailable
    /\ Len(logTerm[leader]) < MaxLogIndex
    /\ \A peer \in activeMembers :
          available[peer] =>
            /\ commitIndex[peer] <= Len(logTerm[leader])
            /\ Prefix(logTerm[peer], commitIndex[peer]) =
               Prefix(logTerm[leader], commitIndex[peer])
    /\ LET newLog == Append(logTerm[leader], term[leader])
           newCommit == Len(newLog)
       IN /\ logTerm' = [peer \in Nodes |->
                 IF peer \in activeMembers /\ available[peer]
                 THEN newLog ELSE logTerm[peer]]
          /\ commitIndex' = [peer \in Nodes |->
                 IF peer \in activeMembers /\ available[peer]
                 THEN newCommit ELSE commitIndex[peer]]
          /\ maxCommitSeen' = [peer \in Nodes |->
                 IF peer \in activeMembers /\ available[peer]
                 THEN newCommit ELSE maxCommitSeen[peer]]
    /\ UNCHANGED <<role, term, maxTermSeen, votedFor, preVotes, votes,
                   available, appliedIndex, snapshotIndex, maxSnapshotSeen,
                   snapshotCluster, activeMembers, membershipEpoch,
                   messages, faultsStopped>>

CatchUp(leader, follower) ==
    /\ leader /= follower
    /\ leader \in activeMembers
    /\ follower \in activeMembers
    /\ available[leader]
    /\ available[follower]
    /\ role[leader] = Leader
    /\ commitIndex[follower] <= Len(logTerm[leader])
    /\ Prefix(logTerm[follower], commitIndex[follower]) =
       Prefix(logTerm[leader], commitIndex[follower])
    /\ logTerm' = [logTerm EXCEPT ![follower] = logTerm[leader]]
    /\ commitIndex' = [commitIndex EXCEPT ![follower] = commitIndex[leader]]
    /\ maxCommitSeen' = [maxCommitSeen EXCEPT ![follower] = commitIndex[leader]]
    /\ appliedIndex' = [appliedIndex EXCEPT
          ![follower] = MinValue(@, commitIndex[leader])]
    /\ UNCHANGED <<role, term, maxTermSeen, votedFor, preVotes, votes,
                   available, snapshotIndex, maxSnapshotSeen,
                   snapshotCluster, activeMembers, membershipEpoch,
                   messages, faultsStopped>>

ApplyOne(n) ==
    /\ n \in activeMembers
    /\ available[n]
    /\ appliedIndex[n] < commitIndex[n]
    /\ appliedIndex' = [appliedIndex EXCEPT ![n] = @ + 1]
    /\ UNCHANGED <<role, term, maxTermSeen, votedFor, preVotes, votes,
                   available, logTerm, commitIndex, maxCommitSeen,
                   snapshotIndex, maxSnapshotSeen, snapshotCluster,
                   activeMembers, membershipEpoch, messages, faultsStopped>>

CreateSnapshot(n) ==
    /\ n \in activeMembers
    /\ available[n]
    /\ appliedIndex[n] > snapshotIndex[n]
    /\ snapshotIndex' = [snapshotIndex EXCEPT ![n] = appliedIndex[n]]
    /\ maxSnapshotSeen' = [maxSnapshotSeen EXCEPT ![n] = appliedIndex[n]]
    /\ snapshotCluster' = [snapshotCluster EXCEPT ![n] = ClusterId]
    /\ UNCHANGED <<role, term, maxTermSeen, votedFor, preVotes, votes,
                   available, logTerm, commitIndex, maxCommitSeen,
                   appliedIndex, activeMembers, membershipEpoch,
                   messages, faultsStopped>>

InstallSnapshot(source, dest) ==
    /\ source /= dest
    /\ source \in activeMembers
    /\ dest \in activeMembers
    /\ available[source]
    /\ available[dest]
    /\ snapshotCluster[source] = ClusterId
    /\ snapshotIndex[source] >= snapshotIndex[dest]
    /\ snapshotIndex[source] <= commitIndex[source]
    /\ snapshotIndex' = [snapshotIndex EXCEPT
          ![dest] = snapshotIndex[source]]
    /\ maxSnapshotSeen' = [maxSnapshotSeen EXCEPT
          ![dest] = snapshotIndex[source]]
    /\ snapshotCluster' = [snapshotCluster EXCEPT ![dest] = ClusterId]
    /\ logTerm' = [logTerm EXCEPT ![dest] = logTerm[source]]
    /\ commitIndex' = [commitIndex EXCEPT ![dest] = commitIndex[source]]
    /\ maxCommitSeen' = [maxCommitSeen EXCEPT ![dest] = commitIndex[source]]
    /\ appliedIndex' = [appliedIndex EXCEPT
          ![dest] = MaxValue(@, snapshotIndex[source])]
    /\ UNCHANGED <<role, term, maxTermSeen, votedFor, preVotes, votes,
                   available, activeMembers, membershipEpoch,
                   messages, faultsStopped>>

RemoveMember(leader, removed) ==
    /\ leader /= removed
    /\ leader \in activeMembers
    /\ removed \in activeMembers
    /\ role[leader] = Leader
    /\ available[leader]
    /\ Cardinality(activeMembers) > Quorum
    /\ activeMembers' = activeMembers \ {removed}
    /\ membershipEpoch' = membershipEpoch + 1
    /\ role' = [role EXCEPT ![removed] = Follower]
    /\ preVotes' = [preVotes EXCEPT ![removed] = {}]
    /\ votes' = [votes EXCEPT ![removed] = {}]
    /\ UNCHANGED <<term, maxTermSeen, votedFor, available, logTerm,
                   commitIndex, maxCommitSeen, appliedIndex, snapshotIndex,
                   maxSnapshotSeen, snapshotCluster, messages, faultsStopped>>

Restart(n) ==
    /\ ~available[n]
    /\ available' = [available EXCEPT ![n] = TRUE]
    /\ role' = [role EXCEPT ![n] = Follower]
    /\ preVotes' = [preVotes EXCEPT ![n] = {}]
    /\ votes' = [votes EXCEPT ![n] = {}]
    /\ UNCHANGED <<term, maxTermSeen, votedFor, logTerm, commitIndex,
                   maxCommitSeen, appliedIndex, snapshotIndex,
                   maxSnapshotSeen, snapshotCluster, activeMembers,
                   membershipEpoch, messages, faultsStopped>>

MakeUnavailable(n) ==
    /\ ~faultsStopped
    /\ available[n]
    /\ available' = [available EXCEPT ![n] = FALSE]
    /\ role' = [role EXCEPT ![n] = Follower]
    /\ UNCHANGED <<term, maxTermSeen, votedFor, preVotes, votes, logTerm,
                   commitIndex, maxCommitSeen, appliedIndex, snapshotIndex,
                   maxSnapshotSeen, snapshotCluster, activeMembers,
                   membershipEpoch, messages, faultsStopped>>

DropMessage(index) ==
    /\ ~faultsStopped
    /\ messages' = RemoveAt(messages, index)
    /\ UNCHANGED <<role, term, maxTermSeen, votedFor, preVotes, votes,
                   available, logTerm, commitIndex, maxCommitSeen,
                   appliedIndex, snapshotIndex, maxSnapshotSeen,
                   snapshotCluster, activeMembers, membershipEpoch,
                   faultsStopped>>

DuplicateMessage(index) ==
    /\ ~faultsStopped
    /\ Len(messages) < MaxMessages
    /\ messages' = Append(messages, messages[index])
    /\ UNCHANGED <<role, term, maxTermSeen, votedFor, preVotes, votes,
                   available, logTerm, commitIndex, maxCommitSeen,
                   appliedIndex, snapshotIndex, maxSnapshotSeen,
                   snapshotCluster, activeMembers, membershipEpoch,
                   faultsStopped>>

StopFaults ==
    /\ ~faultsStopped
    /\ faultsStopped' = TRUE
    /\ available' = [n \in Nodes |-> TRUE]
    /\ UNCHANGED <<role, term, maxTermSeen, votedFor, preVotes, votes,
                   logTerm, commitIndex, maxCommitSeen, appliedIndex,
                   snapshotIndex, maxSnapshotSeen, snapshotCluster,
                   activeMembers, membershipEpoch, messages>>

ConvergeLeader ==
    /\ faultsStopped
    /\ QuorumAvailable
    /\ NoLeader
    /\ \E elected \in activeMembers :
          /\ available[elected]
          /\ term[elected] < MaxTerm
          /\ \A peer \in activeMembers :
                Len(logTerm[elected]) >= Len(logTerm[peer])
          /\ role' = [role EXCEPT ![elected] = Leader]
          /\ term' = [term EXCEPT ![elected] = @ + 1]
          /\ maxTermSeen' = [maxTermSeen EXCEPT ![elected] = term'[elected]]
          /\ votedFor' = [votedFor EXCEPT ![elected] = elected]
          /\ votes' = [votes EXCEPT ![elected] = activeMembers]
    /\ UNCHANGED <<preVotes, available, logTerm, commitIndex,
                   maxCommitSeen, appliedIndex, snapshotIndex,
                   maxSnapshotSeen, snapshotCluster, activeMembers,
                   membershipEpoch, messages, faultsStopped>>

Next ==
    \/ \E n \in Nodes : StartPreVote(n)
    \/ \E n, peer \in Nodes : SendPreVote(n, peer)
    \/ \E index \in 1..Len(messages) : DeliverPreVote(index)
    \/ \E n \in Nodes : StartElection(n)
    \/ \E n, peer \in Nodes : SendVote(n, peer)
    \/ \E index \in 1..Len(messages) : DeliverVote(index)
    \/ \E n \in Nodes : BecomeLeader(n)
    \/ \E n \in Nodes : AppendCommitted(n)
    \/ \E leader, follower \in Nodes : CatchUp(leader, follower)
    \/ \E n \in Nodes : ApplyOne(n)
    \/ \E n \in Nodes : CreateSnapshot(n)
    \/ \E source, dest \in Nodes : InstallSnapshot(source, dest)
    \/ \E leader, removed \in Nodes : RemoveMember(leader, removed)
    \/ \E n \in Nodes : Restart(n)
    \/ \E n \in Nodes : MakeUnavailable(n)
    \/ \E index \in 1..Len(messages) : DropMessage(index)
    \/ \E index \in 1..Len(messages) : DuplicateMessage(index)
    \/ StopFaults
    \/ ConvergeLeader

TypeOK ==
    /\ role \in [Nodes -> {Follower, Candidate, Leader}]
    /\ term \in [Nodes -> 0..MaxTerm]
    /\ maxTermSeen \in [Nodes -> 0..MaxTerm]
    /\ votedFor \in [Nodes -> Nodes \union {Nil}]
    /\ preVotes \in [Nodes -> SUBSET Nodes]
    /\ votes \in [Nodes -> SUBSET Nodes]
    /\ available \in [Nodes -> BOOLEAN]
    /\ logTerm \in [Nodes -> Seq(0..MaxTerm)]
    /\ \A n \in Nodes : Len(logTerm[n]) <= MaxLogIndex
    /\ commitIndex \in [Nodes -> 0..MaxLogIndex]
    /\ maxCommitSeen \in [Nodes -> 0..MaxLogIndex]
    /\ appliedIndex \in [Nodes -> 0..MaxLogIndex]
    /\ snapshotIndex \in [Nodes -> 0..MaxLogIndex]
    /\ maxSnapshotSeen \in [Nodes -> 0..MaxLogIndex]
    /\ snapshotCluster \in [Nodes -> {Nil, ClusterId}]
    /\ activeMembers \subseteq Nodes
    /\ membershipEpoch \in Nat
    /\ messages \in Seq(Message)
    /\ Len(messages) <= MaxMessages
    /\ faultsStopped \in BOOLEAN

AtMostOneLeaderPerTerm ==
    \A observedTerm \in 0..MaxTerm :
        Cardinality({n \in activeMembers :
            role[n] = Leader /\ term[n] = observedTerm}) <= 1

TermsNeverDecrease == \A n \in Nodes : term[n] = maxTermSeen[n]

CommittedIndexNeverDecreases ==
    \A n \in Nodes : commitIndex[n] = maxCommitSeen[n]

CommittedPrefixNeverConflicts ==
    \A left, right \in Nodes :
        LET common == MinValue(commitIndex[left], commitIndex[right]) IN
            Prefix(logTerm[left], common) = Prefix(logTerm[right], common)

AppliedNeverExceedsCommit ==
    \A n \in Nodes : appliedIndex[n] <= commitIndex[n]

SnapshotIdentityMatches ==
    \A n \in Nodes :
        snapshotIndex[n] = 0 \/ snapshotCluster[n] = ClusterId

SnapshotIndexNeverDecreases ==
    \A n \in Nodes : snapshotIndex[n] = maxSnapshotSeen[n]

RemovedNodeCannotRegainAuthority ==
    \A n \in Nodes \ activeMembers : role[n] /= Leader

Spec == Init /\ [][Next]_vars /\ WF_vars(ConvergeLeader)

EventuallyLeaderAfterFaultsStop ==
    (faultsStopped /\ QuorumAvailable) ~> HasLeader

=============================================================================
