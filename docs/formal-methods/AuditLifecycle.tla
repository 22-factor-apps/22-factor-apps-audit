---------------------------- MODULE AuditLifecycle ----------------------------
EXTENDS Naturals

TotalFactors == 22

VARIABLES pending, observed, missing, manualReview, published

vars == <<pending, observed, missing, manualReview, published>>

Init ==
  /\ pending = TotalFactors
  /\ observed = 0
  /\ missing = 0
  /\ manualReview = 0
  /\ published = FALSE

ClassifyObserved ==
  /\ published = FALSE
  /\ pending > 0
  /\ pending' = pending - 1
  /\ observed' = observed + 1
  /\ UNCHANGED <<missing, manualReview, published>>

ClassifyMissing ==
  /\ published = FALSE
  /\ pending > 0
  /\ pending' = pending - 1
  /\ missing' = missing + 1
  /\ UNCHANGED <<observed, manualReview, published>>

ClassifyManualReview ==
  /\ published = FALSE
  /\ pending > 0
  /\ pending' = pending - 1
  /\ manualReview' = manualReview + 1
  /\ UNCHANGED <<observed, missing, published>>

Publish ==
  /\ published = FALSE
  /\ pending = 0
  /\ published' = TRUE
  /\ UNCHANGED <<pending, observed, missing, manualReview>>

Next ==
  \/ ClassifyObserved
  \/ ClassifyMissing
  \/ ClassifyManualReview
  \/ Publish

Spec == Init /\ [][Next]_vars /\ WF_vars(Next)

TypeOK ==
  /\ pending \in 0..TotalFactors
  /\ observed \in 0..TotalFactors
  /\ missing \in 0..TotalFactors
  /\ manualReview \in 0..TotalFactors
  /\ published \in BOOLEAN

ConservesCoverage ==
  pending + observed + missing + manualReview = TotalFactors

PublishedCoverage ==
  published => pending = 0

EventuallyPublished == <>published
===============================================================================
