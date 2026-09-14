# Pre-M3 Remediation Batch D: knowledge chronology and ordered-zone canonicality

**Status:** ADR 0049 accepted after design review approval; final exact-head
evidence review remains required before PR #168 can merge.

## Goal

Close FND-002 and FND-006B through explicit, fail-closed state contracts and
minimal validation changes without changing the V3 wire shape, schema, digest
domain, or valid fixture bytes.

## Approved design

FND-002 uses one chronology for active and retired retained records.
InitialConfiguration is configuration-only and unsequenced. Observed
acquisition is a lower bound. A location fact may equal acquisition only when
its complete provenance is identical, representing the same Acquire
occurrence. That fact may later move unchanged into history or last-known.
Later observed locations are strictly newer. History is oldest-to-newest,
strictly increasing among observed facts, allows gaps, and permits at most one
initial location fact at its earliest position. Invalidation is observed and
strictly later than every prior observed retained fact. All observed values
remain below next_visible_sequence.

FND-006B makes the ordered vector authoritative and ZonePosition a canonical
witness. Vector index zero is top. Every live ordered object uses
Top { offset } equal to its vector ordinal. Unordered objects remain absent
from vectors. Bottom and Index remain represented by the existing type but are
invalid persisted current-state spellings. Retained historical and last-known
ordered locations also use Top; their offsets are preserved without
reconstructing an old vector. Empty ordered-zone entries are invalid and are
rejected, never normalized.

## Components and data flow

The state validator remains the owner:

1. detached M2 knowledge-shape validation applies one shared chronology helper
   to active and retired records;
2. zone validation checks every live vector member against its key and exact
   Top ordinal, rejects empty keys, and retains existing membership checks;
3. retained-location shape validation rejects noncanonical ordered position
   variants in current, historical, and last-known facts;
4. V3 digest and environment checkpoint code remain consumers of validated
   state and receive no new normalization path.

Fixture mutation support removes an ordered-zone map entry when its last live
member leaves, so valid transition products do not create empty authoritative
keys. It does not repair externally supplied malformed states.

## Testing design

State tests first reproduce the accepted BASE behavior for every chronology and
ordered-zone case requested by Batch D. The RED tests assert the approved
post-decision result and are run before production validation changes.

The focused green evidence covers:

- knowledge chronology, lifecycle same-occurrence retention, sequence gaps, and
  future bounds;
- live Top ordinal checks, noncanonical position variants, empty keys, and
  existing membership controls;
- digest rejection for invalid states and digest sensitivity for a valid
  two-object reorder;
- checkpoint construction rejection for an invalid state.

Workspace checks then cover mtgml-state, mtgml-environment,
mtgml-conformance, formatting, check, clippy, tests, and applicable Python,
schema, maintainer, and integration profiles. Wrapper failures caused by the
Windows /bin/bash environment remain BLOCKED and are not promoted to PASS.

## Scope exclusions

This design does not touch FND-007, FND-008, FND-012B, FND-022B/E, FND-026B/C/D,
FND-009+, EVD, HRD, Issue #162, M3, real Magic legality, cards, Card IR,
combat, priority, stack expansion, replay redesign, event delivery, or
performance.

## Review boundary

The candidate is a review vehicle, not a frozen contract. The PR must report
the exact final head, RED and focused evidence, valid fixture digest status,
local wrapper status, hosted CI status, and the remaining independent
exact-head review as the next action.
