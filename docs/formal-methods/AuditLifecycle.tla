---------------------------- MODULE AuditLifecycle ----------------------------
EXTENDS Naturals, FiniteSets, TLC

Factors == 1..22
EvidenceStates == {"Observed", "Missing", "ManualReview"}
FactorSymmetry == Permutations(Factors)

VARIABLES status, published

vars == <<status, published>>

Init ==
  /\ status = [factor \in Factors |-> "Pending"]
  /\ published = FALSE

Classify(factor, evidenceState) ==
  /\ published = FALSE
  /\ factor \in Factors
  /\ status[factor] = "Pending"
  /\ evidenceState \in EvidenceStates
  /\ status' = [status EXCEPT ![factor] = evidenceState]
  /\ UNCHANGED published

Publish ==
  /\ published = FALSE
  /\ \A factor \in Factors : status[factor] \in EvidenceStates
  /\ published' = TRUE
  /\ UNCHANGED status

Next ==
  \/ \E factor \in Factors, evidenceState \in EvidenceStates :
       Classify(factor, evidenceState)
  \/ Publish

Spec == Init /\ [][Next]_vars

TypeOK ==
  /\ status \in [Factors -> (EvidenceStates \union {"Pending"})]
  /\ published \in BOOLEAN

PublishedCoverage ==
  published => \A factor \in Factors : status[factor] \in EvidenceStates

THEOREM Spec => []TypeOK
THEOREM Spec => []PublishedCoverage
===============================================================================
