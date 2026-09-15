# Architecture Decision Records

ADRs are immutable decision history. Superseded records remain and point to replacements. The accepted base sequence currently runs through ADR 0053, and ADR 0000 is the template. ADR 0054 is the acceptance vehicle for the M3 pre-T0 hardening plan: merging PR #184 accepts the plan, while the separate exact-master reauthorization remains the execution gate.

ADR numbers 0042 through 0047 are historically occupied by accepted ContextApplication/M2.5 decisions that were intentionally removed from the active source tree by the post-purge cleanup. Their numbers remain permanently reserved and are not reusable; the acceptance of ADR 0048 is therefore accompanied by an explicit historical numbering gap.

ADRs 0039 and 0040 are accepted M2.A architecture decisions. Their acceptance
froze the implementation direction for M2.B but did not by itself make any
executable M2 behavior gate `PASS`; the later exact M2.Final closure is
recorded by accepted ADR 0041.

ADR 0041 accepts the reviewed capability-oriented semantic-ownership candidate after `M2.Final`; the post-acceptance drift re-review found no material contradiction with the consolidated M2 architecture.

## Reviewed candidates awaiting acceptance

Reviewed ADR candidates may be stored under `docs/adr/candidates/` without allocating a permanent ADR number. They are informative until a later acceptance change assigns the then-current number and changes the record to `Accepted`. ADR 0054 is the current numbered hardening acceptance candidate; its merge accepts the plan, but does not authorize execution.

A candidate must not be cited as accepted architecture, used to claim executable support, or used to begin a later milestone before its explicit acceptance change. Candidate numbering shown inside research material is provisional only.

ADR 0049 accepts the Pre-M3 Batch-D knowledge chronology and ordered-zone
canonicality decision. Its executable evidence remains subject to the exact
head review recorded in the associated pull request.

ADR 0050 accepts the Pre-M3 FND-028 `PlayerId(0)` policy decision as Option A.
It records policy intent only; implementation remains separately gated by the
accepted review and planning process.

ADR 0051 resolves OD-004 by binding the first M3 rules case to the exact
official Wizards TXT artifact recorded in the repository-owned authority
identity record. It does not implement rules or authorize M3.

ADR 0052 resolves the OD-019 M3 deadline as a format-neutral initial M3 with
`FormatState::None`. It deliberately does not freeze a generic format-hook API
or authorize Commander semantics.

ADR 0053 is the accepted M3 Entry Decision. Its companion foundation artifact
freezes the exact capability closure, ownership graph, S1, exclusions, and
bounded exit. Acceptance makes the decision durable on `master`, but does not
authorize M3. Authorization requires the separate exact-`master` review,
Issue #178 gate update, and explicit comment specified by ADR 0053.

ADR 0054 records the M3 Pre-T0 hardening plan. It preserves ADR 0053 and
Foundation V1 as historical evidence, corrects the semantic dependency graph,
defines Foundation V2 and the coordinated V4 state/persistence prerequisite,
and resets current execution authorization. PR #184 is the plan-acceptance
vehicle; a later exact-master review separately authorizes execution.

The merge boundary is explicit:

```text
ADR_0054 = ACCEPTED
FOUNDATION_V2 = ACCEPTED_HARDENED_M3_SCOPE
M3_AUTHORIZED = NO
AUTHORIZATION_HEAD = NOT_SET
```

Create a new record with:

```bash
python scripts/new_adr.py "Title"
```

Resolve an open decision by updating its row in [`../OPEN_DECISIONS.md`](../OPEN_DECISIONS.md), never by deleting it.

An ADR records intent, compatibility and consequences; executable fixtures/conformance still prove behavior.
