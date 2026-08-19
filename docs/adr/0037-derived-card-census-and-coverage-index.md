# ADR 0037: Derived card census and coverage index

- **Status:** accepted
- **Date:** 2026-08-20
- **Supersedes:** none

## Context

Manafold will eventually need trustworthy answers to questions such as:

- how many Magic card rules identities exist in a pinned inventory;
- which identities map to reviewed Manafold definitions;
- which definitions are implemented, covered, or certified in a bundle;
- which reviewed capabilities block further coverage;
- how source and project coverage change between snapshots and commits.

A global card inventory is useful for maintainer planning, but it must not become a second rules authority, a global certification claim, or a runtime dependency.

## Decision

Manafold may maintain a **Card Census / Coverage Index** as a deterministic, non-authoritative read model over pinned source artifacts and repository state.

The dependency direction is one way:

```text
pinned source artifacts      Manafold repository
          \                      /
           \                    /
            deterministic builder
                    |
                    v
          census/read-model output

RulesKernel       -X-> census
PlayerEndpoint    -X-> census
certification     -X-> census as authority
```

The census reports facts derived from authoritative inputs; it does not define gameplay semantics, legality, capability truth, or certification truth.

## Card identity and counting

There is no unqualified global `total_cards` metric.

The primary gameplay-coverage denominator is the number of unique gameplay rules identities in an explicitly identified inventory snapshot and scope.

For an inventory seeded from Scryfall, a provider Oracle/rules identity is normally the source identity, subject to explicit handling of provider-specific exceptions.

Names, faces/components, printings, tokens, emblems, generated/reference objects, and Manafold `CardDefinitionId` values are separate metrics. Names are never identity keys.

## Source authority

Scryfall bulk data is accepted as a practical **inventory and reconciliation input**. Scryfall identifiers are authoritative only for Scryfall's own identity namespace.

Scryfall is **not** Manafold's final authority for Magic semantics. Certified behavior continues to use the applicable official rules, Oracle/card source, rulings, format policy, legality policy, and Manafold interpretation records according to the existing authority policy.

Source acquisition is separate from census construction. Deterministic builds consume already-pinned source artifacts and do not depend on live network responses.

## Support and certification semantics

The census does not introduce one combined lifecycle.

It preserves the distinction between source presence, source-to-definition mapping, the existing card lifecycle, the existing capability lifecycle, evidence, capability closure, and bundle-specific certification.

The read model must not contain manually maintained support flags that duplicate repository authority. Support state is derived from validated repository artifacts.

`Imported != Supported`, `Parsed != Supported`, and `Implemented != Certified` remain unchanged.

Certification is never flattened into a global boolean. A report may state that a definition is certified in one or more matching bundles, but may not infer that the card is globally certified.

## Capability-gap analysis

Capability-gap reporting is maintainer planning information, not support evidence.

Exact capability-impact claims may use only complete reviewed requirements. Parser-, generator-, or LLM-derived classifications remain `EXPERIMENTAL` until reviewed and must not silently enter exact support or certification counts.

The concrete metric vocabulary, blocker algorithms, and implementation schema are deferred to the census implementation specification.

## Derived storage and rebuildability

The census uses the smallest practical derived read model. SQLite is the preferred initial implementation, but storage technology is not part of the authoritative architecture and may be replaced without changing gameplay semantics.

Generated census storage:

- is disposable and rebuildable from pinned inputs;
- is not committed as source authority;
- is not queried by semantic runtime code;
- may be deleted without losing authoritative information.

Any persistent census build identity or digest contract is defined when the corresponding artifact schema is specified. Raw SQLite file bytes are not implicitly a semantic contract.

## Presentation boundary

Presentation metadata is outside semantic coverage identity.

Image URLs, preferred artwork, CDN/R2 keys, cache status, prices, popularity ranks, and similar presentation fields may be handled by adjacent tooling, but presentation changes must not create gameplay-support claims.

A future website may consume presentation data derived from the same source identities without creating a second rules engine.

## Failure principle

Ambiguous or internally inconsistent census inputs fail closed rather than producing authoritative-looking coverage claims.

Detailed validation errors, partial-build behavior, source migration handling, and diagnostic output belong to the implementation specification.

## Scope and staging

This decision does **not** add a gate to M1 and does not unblock real card expansion.

Recommended staging:

```text
now
  architecture decision only

early tooling, when useful
  pinned source inputs
  minimal inventory index
  basic reports

M2.5
  locked-deck mapping and exact capability-closure integration

M3/M4
  reviewed requirement profiles and trustworthy capability-gap prioritization

later
  optional static/public coverage presentation
```

The initial implementation should remain deliberately small. The larger research schema is a design reference, not a V1 requirement.

## Consequences

Positive:

- Manafold can measure inventory and project coverage without widening support claims;
- source drift and definition coverage can become auditable;
- capability work can later be prioritized using reviewed evidence;
- generated caches remain cheap to delete and rebuild;
- future CLI/reporting or website presentation does not couple to the RulesKernel.

Costs and constraints:

- exact global capability-impact counts require reviewed classification work;
- source retention and redistribution need an explicit policy before public distribution;
- source-provider format changes require importer maintenance;
- reports must state their snapshot, scope, denominator, and review basis.

## Rejected alternatives

- live Scryfall/API queries from the RulesKernel;
- treating Scryfall as final Magic semantic authority;
- name-based card identity;
- one combined source/support/certification lifecycle;
- a committed derived database as canonical truth;
- parser confidence as support evidence;
- a global `certified=true` card flag;
- making the global census an M1 acceptance gate;
- freezing a detailed database schema or capability-impact algorithm in this ADR.

## Review trigger

Revisit this ADR if evidence shows that the read model must participate in authoritative semantics, the primary coverage identity is inadequate for real source data, or the derived-storage boundary prevents required content-maintenance workflows.
