# Proposed ADR: Pre-M3 Batch D knowledge chronology and ordered-zone canonicality

- **Status:** PROPOSED
- **Date:** 2026-09-14
- **Scope:** FND-002 and FND-006B only
- **Review vehicle:** Pre-M3 Remediation Batch D pull request
- **Acceptance condition:** independent exact-head review and merge; this proposal is not accepted architecture

## Context

Batch A established the local state-closure checks for retained knowledge and
ordered-zone membership. It deliberately left two cross-field contracts open:

1. retained knowledge validates each provenance and the local order of observed
   history, but does not validate one chronology across acquisition, retained
   locations, and retirement invalidation;
2. ZoneState.ordered_zones and ZoneLocation.position both contribute to
   current V3 state identity, but the accepted M1.1 contract does not define
   how their order representations relate.

The current lifecycle already supplies important evidence. Acquire may create
an acquisition provenance and an initial known-location fact from one visible
occurrence. UpdateLocation moves that exact current fact into history and
creates a strictly later observed current fact. Invalidate retires the prior
current fact and creates an observed invalidation from the next occurrence.
Every observed provenance remains below next_visible_sequence.

The current reset and conformance fixtures use Top { offset } with vector
ordinal semantics. The Bottom and Index variants have no positive current
fixture or production state-construction use. The V3 digest includes both
representations, so accepting aliases would allow multiple digest-significant
states for one semantic order.

## Decision

### FND-002: retained-knowledge chronology

The retained record is read as one chronology from earliest to latest. This
contract applies identically to active records and retired records; the retired
record adds one later invalidation.

#### Acquisition

- InitialConfiguration is valid for acquisition in reset/configuration state.
  It has no numeric sequence.
- An Observed acquisition is valid only with an accepted channel/cause and a
  sequence below next_visible_sequence.
- A lifecycle-created acquisition is always observed. A visible lifecycle
  occurrence cannot create InitialConfiguration provenance.
- An observed acquisition is a lower temporal bound for every later retained
  observed fact.

#### Location facts and same-occurrence equality

- The historical-location vector is ordered from oldest to newest.
- Observed historical sequences are strictly increasing. Sequence numbers need
  not be contiguous.
- A retained location fact may have the same observed sequence as an observed
  acquisition only when its complete provenance equals the acquisition
  provenance. This represents the location fact created by the same Acquire
  occurrence; equality is not a general uniqueness rule.
- That acquire-created location fact may later be in known_location,
  historical_locations, or last_known_location without changing its
  provenance.
- Every later observed location fact is strictly newer than that fact and than
  the preceding observed history fact.
- At most one retained location fact per record may use
  InitialConfiguration. If present, it is the earliest retained location fact.
  An initial location fact may not appear after an observed history fact. A
  current or last-known initial location fact is valid only when no later
  retained location fact exists.
- An observed location fact older than an observed acquisition is invalid.
  Equality with acquisition requires complete provenance equality as above.

For an active record, an observed current location must be strictly newer than
the newest observed historical location. If no historical observed fact exists,
it may equal acquisition only under the same complete-provenance equality rule;
otherwise it must be newer than observed acquisition. A current
InitialConfiguration fact is valid only as the record's sole retained location
fact.

For a retired record, last_known_location follows exactly the same rules as
the active current location. Retirement does not rewrite location provenance.

#### Invalidation

- Invalidation is always Observed; InitialConfiguration invalidation is invalid.
- Its sequence is strictly newer than:
  - observed acquisition, when acquisition is observed;
  - every observed historical location fact; and
  - observed current or last-known location, when present.
- Invalidation is the provenance of the retirement occurrence. It does not
  share the sequence of the retained fact that was last current.

All observed provenances, including invalidation, remain strictly below
next_visible_sequence. No visible-sequence contiguity is required.

### FND-006B: ordered-zone canonicality

ZoneState.ordered_zones is authoritative for semantic order. The
ZoneLocation.position field is a canonical redundant witness validated
against that vector.

- Vector index 0 is the top of the ordered zone.
- For every live ordered object at vector ordinal i, its location is exactly
  ZonePosition::Top { offset: i }.
- Top.offset is zero-based and must fit the vector ordinal; therefore it is
  strictly less than the vector length for a live ordered object.
- ZonePosition::Unordered is valid only for an object absent from every ordered
  vector.
- ZonePosition::Bottom and ZonePosition::Index remain in the Rust/Serde type
  for compatibility, but neither is a valid canonical persisted spelling in
  the current EngineState. They are rejected rather than reinterpreted.
- Every persisted ordered ZoneLocation inside EngineState, including a
  retained historical or last-known location, uses Top { offset }. Historical
  facts retain their numeric top offset as evidence; the validator does not
  reconstruct or compare their old zone vector.
- An ordered_zones map entry must contain at least one object. An empty entry
  has no semantic meaning and is rejected as invalid state. Validation never
  removes it.
- Existing membership, uniqueness, key, player-reference, and unordered-object
  checks remain in force.

This is structural state validation. It does not assert Magic legality for any
particular ZoneKind.

## Rejected alternatives

### Leave chronology local

Rejected because a record could retain an acquisition after its history, a
current fact older than history, or an invalidation before retirement facts
while passing all existing local checks.

### Require globally unique provenance sequences

Rejected because Acquire intentionally binds acquisition and its initial
known-location fact to the same visible occurrence. The same fact may later be
retained in history without acquiring a new sequence.

### Make ZonePosition authoritative

Rejected because the vector is already the serialized semantic zone order and
is used directly by lifecycle/conformance fixtures. Making the vector derived
would require a broader state-ownership and transition audit.

### Accept all position variants with equivalence checks

Rejected because Top { 0 }, Index { 0 }, and a corresponding bottom offset
can denote one slot while remaining distinct digest inputs. Exact consistency
without a canonical spelling would preserve the duplicate-representation risk.

### Remove or reinterpret Bottom and Index

Rejected for this batch. The variants remain present in the type and wire
shape. This proposal only rejects them in the current canonical state; any
future accepted meaning requires a separately versioned contract decision.

### Normalize empty ordered-zone entries

Rejected because validation must fail closed and must not mutate or canonicalize
malformed authoritative state.

### Add Magic zone legality

Rejected because validate_engine_state() owns generic structural closure. M3
owns actual Magic zone semantics.

## Compatibility impact

The Rust enum and Serde shape do not change. The accepted current-state set is
strengthened: previously accepted malformed chronology, noncanonical position,
and empty-key states now fail validation. Valid existing canonical states retain
their representation and digest bytes.

PUBLIC_API_CHANGE = NO
WIRE_CHANGE = NO
SCHEMA_CHANGE = NO
DIGEST_DOMAIN_CHANGE = NO
VALID_FIXTURE_DIGEST_CHANGE = NO

## Digest implications

FullStateDigestV3 continues to validate EngineState before constructing its
input. A malformed chronology, Bottom/Index persisted position, ordinal
mismatch, or empty ordered-zone key therefore cannot obtain a current V3
digest. The digest writer and domain remain unchanged.

The canonical reset known answer remains the regression for unchanged valid
bytes. Digest sensitivity for order is tested with two valid objects whose
canonical vector order and matching Top offsets are swapped. The old Bottom
mutation becomes a fail-closed negative test rather than a valid mutation.

## Replay and checkpoint implications

No replay or checkpoint wire format changes. Existing valid states continue
through V3 checkpoint construction, restore, fork, and replay with the same
identity. Invalid states fail before digest or checkpoint construction. No
historical V1/V2 meaning is reinterpreted and no migration is introduced.

## Information-safety implications

The chronology checks preserve the exact provenance values and do not infer
hidden events or fill sequence gaps. The ordered-zone rule removes
digest-significant aliases without exposing hidden order through projection.
Validation and projection remain read-only; no normalization, allocation, or
new information channel is added.

## M3 implications

This proposal does not add real Magic rules, zone legality, cards, Card IR,
combat, priority, stack expansion, or event-delivery behavior. M3 may use the
canonical ordered-state representation but must make any additional
zone-specific semantics explicit in its own reviewed scope.

M3_STARTED = NO
M3_AUTHORIZED = NO

## Migration and versioning implications

No migration is defined. Existing ZonePosition variants are not removed from
the type or reinterpreted in place. If a later milestone needs persisted
bottom-relative or arbitrary-index semantics, it must propose a versioned state
and digest decision before accepting those forms. The same rule applies if a
future chronology needs to represent more than the current lifecycle can
encode.

## Acceptance tests

The Batch-D evidence must show:

1. every required FND-002 BASE case, with the pre-fix result recorded as
   ACCEPTED or REJECTED;
2. the RED cases fail for the new contract before the validator fix;
3. acquisition/current same-occurrence equality is accepted;
4. Acquire followed by UpdateLocation retains the acquisition fact in history
   with its original sequence and accepts the resulting state;
5. initial acquisition plus initial-prefix/observed history and legal sequence
   gaps are accepted;
6. all invalid chronology cases are rejected without state mutation;
7. live ordered vectors and Top ordinals are cross-validated in linear time;
8. retained historical/last-known ordered locations reject Bottom and Index;
9. empty ordered-zone keys are rejected without mutation;
10. existing unordered/missing/duplicate membership controls remain green;
11. invalid V3 digest and checkpoint construction fail closed;
12. the canonical reset digest bytes remain unchanged and a valid two-object
    order change produces a different digest.

Acceptance of this candidate remains separate from passing these tests.
