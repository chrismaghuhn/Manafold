# ADR 0037: Derived card census and coverage index

- **Status:** accepted
- **Date:** 2026-08-20
- **Supersedes:** none

## Context

Manafold will eventually need a trustworthy answer to questions such as:

- how many Magic card rules identities exist in a pinned inventory;
- which identities map to reviewed Manafold definitions;
- which definitions are implemented, covered, or certified in a bundle;
- which reviewed capabilities block further coverage;
- how source and project coverage change between snapshots and commits.

A global card inventory is useful for maintainer planning, but it must not become a second rules authority, a global certification claim, or a runtime dependency. Manafold already defines gameplay support through reviewed definitions, recursive capability closure, pinned authority snapshots, conformance evidence, and locked bundle certification.

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

The census reports facts derived from authoritative inputs; it does not define gameplay semantics.

## Card counting

There is no unqualified global `total_cards` metric.

The primary gameplay-coverage denominator is:

> the number of unique gameplay rules identities in an explicitly identified inventory snapshot and scope.

For an inventory seeded from Scryfall, a provider Oracle/rules identity is normally the source identity, subject to explicit handling of provider-specific exceptions.

Secondary metrics remain separate, including:

- unique names;
- faces/components;
- physical/digital printings;
- tokens;
- emblems;
- other generated/reference objects;
- Manafold `CardDefinitionId` values.

Names are never identity keys.

## Source authority

Scryfall bulk data is accepted as a practical **inventory and reconciliation input**. Scryfall identifiers are authoritative only for Scryfall's own identity namespace.

Scryfall is **not** Manafold's final authority for Magic semantics. Certified behavior continues to pin the applicable official rules, Oracle/card source, rulings, format policy, legality policy, and Manafold interpretation records according to the existing authority policy.

Source acquisition is separate from deterministic census construction:

```text
networked fetch/pin
  -> exact payload + checksum + source manifest

offline census build
  -> validate pinned inputs
  -> derive index and reports
```

A deterministic build must not fetch live network data.

## Support and lifecycle semantics

The census does not introduce one combined lifecycle.

It preserves distinct facets such as:

- source presence/normalization;
- source-to-definition mapping;
- the existing card lifecycle;
- the existing capability lifecycle;
- capability-closure status;
- evidence status;
- bundle-specific certification.

The database/read model must not contain manually maintained claims such as `implemented=true` that duplicate repository authority. Support state is derived from validated repository artifacts.

`Imported != Supported`, `Parsed != Supported`, `Implemented != Certified` remain unchanged.

## Certification

Certification is never flattened into a global boolean.

The meaningful relation remains equivalent to:

```text
definition
+ bundle identity/content digest
+ authority snapshot set
+ engine/backend identity
+ evidence artifact
-> certification status
```

A report may state that a definition is certified in one or more matching bundles, but may not infer that the card is globally certified.

## Capability-gap analysis

Exact roadmap-impact metrics use only **complete reviewed capability requirements**.

Parser-, generator-, or LLM-derived requirement classifications are `EXPERIMENTAL` planning metadata until reviewed. They cannot enter exact support or certification counts silently.

The census distinguishes at least:

```text
direct_requires(X)
transitive_requires(X)
blocked_by(X)
sole_remaining_blocker(X)
```

A claim that implementing capability `X` "unlocks N cards" is permitted only when, for those N reviewed cards:

- requirement classification is complete;
- `X` and its required dependency work clear the remaining capability blockers;
- no other known mapping, definition, generated-object, native-executor, source, or evidence blocker remains for the stated target stage.

Cards with multiple blockers receive no immediate-unlock credit for clearing only one blocker.

## Storage and rebuildability

The first implementation should use the smallest practical derived read model. SQLite is accepted as the initial local implementation because it is embedded and sufficient for the expected corpus size, but SQLite is **not** an architectural forever-choice.

The generated database:

- is disposable and rebuildable;
- is not committed as source authority;
- is not queried by semantic runtime code;
- is not itself the canonical dataset identity;
- may be deleted without losing authoritative information.

Canonical inputs include the pinned source identities/digests, relevant Manafold repository commit and content artifacts, and the census builder/schema identities.

If a census build needs a stable digest, it hashes a versioned canonical logical representation of its inputs/output facts rather than relying on raw SQLite file bytes.

## Presentation data

Presentation metadata is outside semantic coverage identity.

Image URLs, preferred artwork, CDN/R2 keys, cache status, prices, popularity ranks, and similar presentation fields may be indexed by adjacent tooling, but changing them must not change a semantic card-coverage build solely because presentation changed.

A future website may consume a presentation catalog derived from the same source identities without creating a second rules engine.

## Failure behavior

Structural ambiguity fails closed. Examples include:

- duplicate source rules identities within one declared snapshot;
- ambiguous active source-to-definition mappings;
- reused snapshot identity with different payload bytes;
- malformed required source records;
- invalid capability dependency graphs;
- certification artifacts that reference inconsistent identities.

Expected content gaps such as a missing capability are reported as blockers, not silently approximated.

A blocked diagnostic build may preserve useful diagnostics, but must not present misleading coverage percentages as authoritative results.

## Scope and staging

This decision does **not** add a gate to M1 and does not unblock real card expansion.

Recommended staging:

```text
now
  architecture decision only

early tooling, when useful
  pinned source manifest/fetcher
  minimal deterministic inventory index
  basic summary/show/diff reports

M2.5
  locked-deck mapping and exact capability-closure integration

M3/M4
  reviewed global requirement profiles and trustworthy capability-gap prioritization

later
  optional static/public coverage presentation
```

The initial implementation should remain deliberately small. The research model's larger relational schema is a long-term option, not a requirement for V1.

## Consequences

Positive:

- Manafold can measure global inventory and project coverage without widening support claims;
- source drift and definition coverage become auditable;
- capability work can later be prioritized using reviewed evidence;
- the index can serve CLI/reporting and future website presentation without coupling to the kernel;
- generated caches remain cheap to delete and rebuild.

Costs and constraints:

- exact global capability-impact counts require substantial human-reviewed classification work;
- source payload retention and redistribution need an explicit policy before public distribution;
- source-provider format changes require importer maintenance;
- reports must always state their snapshot, scope, denominator, and review basis.

## Rejected alternatives

- live Scryfall/API queries from the RulesKernel;
- treating Scryfall as final Magic semantic authority;
- name-based card identity;
- one combined source/support/certification lifecycle;
- a committed SQLite database as canonical truth;
- PostgreSQL, Redis, Elasticsearch, or distributed infrastructure for V1;
- parser confidence as support evidence;
- a global `certified=true` card flag;
- making the global census an M1 acceptance gate.

## Review trigger

Revisit this ADR if evidence shows that the read model must participate in authoritative semantics, the primary coverage identity is inadequate for real source data, or the initial storage/rebuild boundary prevents required M2.5+ workflows.
