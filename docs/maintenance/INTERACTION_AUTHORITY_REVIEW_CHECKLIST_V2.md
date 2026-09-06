# Interaction Authority Review Checklist V2

**Status:** accepted V2 checklist definition

**Stability:** accepted

**Checklist ID:** `interaction-authority-review-checklist.v2`

This checklist definition is immutable after admission. It inherits every
semantic obligation of `interaction-authority-review-checklist.v1`; V1 keeps
its historical meaning and is never overwritten or repurposed.

## Required review

The reviewer must perform every V1 check, including exact source and locator
verification, theorem/application identity, finite membership, per-member
preconditions, Candidate/SourceInstance/B2/B1.Final evidence, information
safety, exact source closure, immutable provenance, and same-kind lifecycle
review.

For every ContextApplicationV2 member, the reviewer must inspect:

- the exact historical `source_value` for all ten context slots;
- the exact `reviewed_value` for all ten context slots;
- the exact reviewed value for all four temporal slots;
- the mechanically derived `exact_match` versus `reviewed_divergence` relation;
- positive evidence for every reviewed context value and every temporal value;
- positive evidence even when the reviewed value is `not_applicable`;
- the exact historical source binding and actual inequality for every
  `reviewed_divergence`;
- exact Candidate and SourceInstance binding;
- exact preservation of every V1 precondition;
- exact V3 reviewer-roster and source closure; and
- every mandatory reviewer role under the bound roster.

The reviewer must reject all of the following reasoning shortcuts:

- SourceContext rewrite or normalization;
- lexical inference;
- capability-name inference;
- co-occurrence inference;
- absence-of-evidence inference; and
- any semantic conclusion derived only from rationale, filenames, or prose.

Evidence must be positive, source-bound, and independently resolvable. An
evidence reference proves integrity and locator resolution; it does not prove
substantive human-review sufficiency.

## Information and visibility

The V2 acceptance policy always requires an `information_safety_reviewer`.
The typed information-sensitivity inventory is diagnostic and may support a
future narrower policy, but it cannot waive this role in V2.

## Solo separate self-review

For `solo_separate_self_review`, the reviewer must:

- perform a separate pass after proposal or artifact generation;
- review frozen exact bytes, identities, and source bindings;
- make no semantic edits during the acceptance pass;
- restart the pass after any required edit;
- rerun the complete checklist; and
- record portable review evidence.

The persisted V3 event does not prove timestamps, authoring-pass identity,
reviewer independence, or temporal separation. Solo mode never waives a role
or evidence requirement.

## Review mode and role policy

`multi_reviewer` is a closed persisted mode. It does not imply a minimum
reviewer count unless a later versioned contract adds that rule.

Every V2 application admission requires:

```text
architecture_maintainer
rules_authority_maintainer
conformance_maintainer
information_safety_reviewer
```

`project_owner` is optional. When a selected reviewer has that role in the
bound roster, the event must preserve the reviewer's complete roster role
tuple. No event-local role escalation or role waiver is permitted.

## Supersession and revocation

The reviewer must inspect future supersession and revocation obligations,
including immutable accepted provenance and the prohibition on mutating an
accepted record. This checklist does not validate the supersession graph,
replacement currentness, revocation graph, or current-record selection.
