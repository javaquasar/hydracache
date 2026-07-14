------------------------- MODULE RaftElectionCanary -------------------------
EXTENDS Naturals, FiniteSets

CONSTANTS Nodes, Follower, Leader
VARIABLES role, term
vars == <<role, term>>

Init == /\ role = [n \in Nodes |-> Follower]
        /\ term = [n \in Nodes |-> 1]

ElectFirst(n) ==
    /\ ~\E leader \in Nodes : role[leader] = Leader
    /\ role' = [role EXCEPT ![n] = Leader]
    /\ UNCHANGED term

UnsafeSecondLeader(n) ==
    /\ role[n] = Follower
    /\ \E leader \in Nodes : role[leader] = Leader /\ leader /= n
    /\ role' = [role EXCEPT ![n] = Leader]
    /\ UNCHANGED term

Next == \/ \E n \in Nodes : ElectFirst(n)
        \/ \E n \in Nodes : UnsafeSecondLeader(n)

AtMostOneLeaderPerTerm ==
    \A observedTerm \in {1} :
        Cardinality({n \in Nodes :
            role[n] = Leader /\ term[n] = observedTerm}) <= 1

Spec == Init /\ [][Next]_vars

=============================================================================
