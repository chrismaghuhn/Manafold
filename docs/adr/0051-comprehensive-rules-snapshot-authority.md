# ADR 0051 — Comprehensive Rules Snapshot Authority for Initial M3

- **Status:** accepted
- **Date:** 2026-09-15
- **Owners:** rules maintainers; architecture maintainers; conformance maintainers
- **Resolves:** OD-004
- **Supersedes:** none
- **Superseded by:** none
- **Review provenance:** accepted in the Final Pre-M3 Governance Cleanup PR candidate; the repository-owned snapshot identity and official-source metadata are recorded in [`COMPREHENSIVE_RULES_ARTIFACT_RESEARCH_2026-09-15.md`](../rules/COMPREHENSIVE_RULES_ARTIFACT_RESEARCH_2026-09-15.md)
- **Implementation evidence:** `NOT_RUN`; this ADR changes no rules behavior and authorizes no capability implementation

## Context

The first real M3 rules case needs an immutable external authority identity.
An unversioned link to the current Wizards page is insufficient because a
later rules release must not silently reinterpret old conformance evidence.
The repository must bind an exact official artifact without redistributing the
copyrighted Comprehensive Rules document.

The existing authority policy already keeps Comprehensive Rules, Oracle/card
source, official rulings, format policy, and banlist/deck-legality policy as
separate identities. This decision supplies the missing Comprehensive Rules
snapshot identity for the first M3 case.

## Decision

### Selected authority

For the beginning of M3, Manafold selects the official Wizards of the Coast
Comprehensive Rules **TXT** artifact exposed by the official rules page:

- Publisher: Wizards of the Coast
- Official source page: <https://magic.wizards.com/en/rules>
- Official artifact URL: <https://media.wizards.com/2026/downloads/MagicCompRules%2020260819.txt>
- Official document/file name: `MagicCompRules 20260819.txt`
- Document title: `Magic: The Gathering Comprehensive Rules`
- Effective/document date: **2026-08-07** (August 7, 2026)
- Retrieval date/time: `2026-09-15T17:25:45.1489379Z`
- Media type: `text/plain`
- Exact byte length: `977822`
- SHA-256: `4381ad1b39ab2c05f7d03633a20f711ed37277074d3266dcba5f38cbb527423f`

The stable project-facing snapshot identifier is:

```text
wotc-cr-2026-08-07-txt-20260819-sha256-4381ad1b39ab2c05f7d03633a20f711ed37277074d3266dcba5f38cbb527423f
```

Wizards exposes official DOCX and PDF variants as well. Their exact metadata
and digests are preserved in the repository-owned authority record, but they
are presentation/audit variants for this decision, not separate semantic
rule versions. The TXT variant is selected because it is the direct textual
representation for deterministic clause extraction and rule-number citation.

### Citation policy

Every future rules or capability conformance case must cite:

1. this exact rules snapshot identifier;
2. the applicable Comprehensive Rules rule/section number(s); and
3. any additional official ruling or release-note authority required by the
   declared scope, using that authority's own exact identity.

The snapshot identity alone does not prove a rule, capability, or interaction
correct. Conformance evidence must still execute against the real Rust kernel
and remain bounded by the reviewed case scope.

### Update and migration policy

A new Wizards Comprehensive Rules release does not silently reinterpret old
evidence:

```text
old conformance evidence -> remains bound to its old snapshot
new semantic work        -> explicitly selects a current snapshot
snapshot migration       -> reviewed impact analysis and revalidation
```

Migration analysis must identify affected rule citations, cases, expected
outcomes, capability versions, and dependent certification evidence. A new
snapshot identity is required when the exact official artifact or its digest
changes.

### Separation of authorities

The selected Comprehensive Rules snapshot remains distinct from:

```text
Comprehensive Rules snapshot
    != Oracle/card snapshot
    != official card rulings or release notes
    != format-policy snapshot
    != banlist/deck-legality snapshot
```

Those authorities must be pinned separately when a conformance scope needs
them. A format policy cannot silently rewrite the core rules authority.

### Distribution and repository identity

Manafold stores the official locator, metadata, and digest in its small
repository-owned authority record. It does not vendor the full rules document
or claim redistribution rights that have not been established. Maintainers
can acquire the official artifact locally and verify the exact byte identity
before using it for conformance work.

## M3 consequence

The selected snapshot is sufficient authority for the **first real M3 rules
case**. This ADR:

- implements no Magic rule or mechanic;
- proves no rule correct;
- does not select M3.S1;
- does not authorize M3;
- does not register or certify a capability; and
- does not change state, Decision, observation, wire, schema, replay,
  checkpoint, RNG, digest, or Python semantics.

```text
OD_004 = RESOLVED
M3_STARTED = NO
M3_AUTHORIZED = NO
AUTHORIZATION_HEAD = NOT_SET
```

## Rejected alternatives

### Bind only the live rules page

Rejected because its content can change without giving old evidence an
immutable identity.

### Vendor the complete rules document

Rejected because the task does not establish public redistribution rights and
the repository only needs an exact identity that maintainers can verify.

### Use a third-party mirror or another engine

Rejected because mirrors and external engines are discovery or differential
references only. Wizards is the authority for the selected rules artifact.

### Treat PDF, DOCX, and TXT as separate rules versions

Rejected because the official page exposes them as variants of the same current
document. Manafold selects one exact textual artifact and retains the other
official identities only for audit/cross-reference purposes.

## Compatibility

This is a documentation and authority-identity decision. It changes no
serialized bytes, schema, digest domain, replay/checkpoint meaning, runtime
API, capability registry entry, Card IR, or executable behavior. A future
rules case must still record the selected snapshot identity in its own
conformance evidence.
