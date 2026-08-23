# Architecture Decision Records

ADRs are immutable decision history. Superseded records remain and point to replacements. Numbered accepted records currently run through ADR 0040; ADR 0000 is the template.

ADRs 0039 and 0040 are accepted M2.A architecture decisions. Their acceptance freezes the implementation direction for M2.B, but does not make any executable M2 behavior gate `PASS`; those gates remain `NOT_RUN` until their declared evidence executes.

## Reviewed candidates awaiting acceptance

Reviewed ADR candidates may be stored under [`candidates/`](candidates/) without allocating a permanent ADR number. They are informative until a later acceptance change assigns the then-current number and changes the record to `Accepted`.

Current reviewed candidate:

- [`Capability-Oriented Semantic Domains and Explicit Semantic Ownership`](candidates/capability-oriented-semantic-domains-and-explicit-semantic-ownership.md) — `PROPOSED / NOT ACCEPTED`; independent review found `0 BLOCKER / 0 MAJOR`; recommended acceptance window is after successful `M2.Final` and before M2.5 implementation.

A candidate must not be cited as accepted architecture, used to claim executable support, or used to begin a later milestone before its explicit acceptance change. Candidate numbering shown inside research material is provisional only.

## Creating an accepted ADR

Create a new numbered record with:

```bash
python scripts/new_adr.py "Title"
```

Before acceptance, verify the then-current ADR index, resolve any drift against accepted contracts, assign the actual next number, and update every affected normative/informative reference in the same change.

Resolve an open decision by updating its row in [`../OPEN_DECISIONS.md`](../OPEN_DECISIONS.md), never by deleting it.

An ADR records intent, compatibility and consequences; executable fixtures/conformance still prove behavior.
