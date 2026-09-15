# PlayerId Zero Policy — ADR Candidate

**Status:** superseded working candidate, non-authoritative; proposed ADR 0050 carries the current decision record
**Stability:** provisional, non-authoritative
Owner: architecture maintainers
Reviewers: model/state, replay, observation, and environment maintainers
Proposed ADR: [0050 — `PlayerId(0)` as a valid declared player identity](../../adr/0050-player-id-zero-policy.md)

## Context

`PlayerId` is a generic canonical unsigned identifier and can currently
represent `0`. The executable system does not apply one coherent meaning to
that value: some authoritative and player-facing surfaces accept a declared
zero player, while Replay V3 explicitly rejects a zero step actor. Batch FND-028
therefore remains `BLOCKED_CONTRACT_AMBIGUITY`. This candidate records the
choice that must be reviewed; it does not change any parser, validator,
fixture, schema, replay meaning, or runtime boundary.

This file is retained as the initial Batch-F input. It is intentionally not a
complete current-source matrix; the expanded characterization and selected
policy are recorded in proposed ADR 0050.

## Initial Batch-F executable matrix

| Surface | Current zero behavior | Contract status in Batch F |
| --- | --- | --- |
| Generic `PlayerId` parse/model representation | accepts canonical `0` | unresolved; preserve |
| `EngineState.core.players` | accepts zero when declared as a map key | unresolved; preserve |
| `SyntheticResetInputs` | accepts zero when distinct from the other player | unresolved; preserve |
| `active_player`, `priority_player` | accepts zero when present in the player map | unresolved; preserve |
| object owner/controller | accepts zero when declared | unresolved; preserve |
| pending decision actor | accepts zero when declared | unresolved; preserve |
| `SelectPlayer` candidate | accepts zero when declared | unresolved; preserve |
| knowledge player keys | accepts zero when state coverage is coherent | unresolved; preserve |
| perspective-identity player keys | accepts zero when state coverage is coherent | unresolved; preserve |
| `PlayerInformationStateV2` perspective | local DTO/schema accepts zero | unresolved; preserve |
| observed-event player/actor fields | local DTO/schema accepts zero | unresolved; preserve |
| Replay V3 manifest deck player | accepts zero in current validators | unresolved; preserve |
| Replay V3 step actor | explicitly rejects zero | contradictory signal; preserve |
| environment `bind_player(0)` | accepts a declared zero player | unresolved; preserve |
| wire/schema unsigned player fields | schema permits zero | preserve historical/executable shape |

## Considered options

### A. Zero is valid everywhere

Make every model, authoritative, replay, observation, environment, and player
boundary treat `PlayerId(0)` as an ordinary player. This is the least
restrictive interpretation, but it must explain and migrate the current Replay
V3 step-actor rejection without silently changing historical replay meaning.

### B. Zero is forbidden at authoritative/player-identity boundaries

Keep generic unsigned parsing able to represent zero, but reject zero wherever
the value denotes an actual player in authoritative state, decisions, replay,
knowledge, perspective identity, observation, or environment binding. This
requires a reviewed boundary matrix and compatibility treatment for existing
fixtures and any historical data that uses zero.

### C. Zero is a defined reserved/sentinel value

Assign zero a precise non-player meaning and prohibit it from ordinary player
identity positions. This requires every affected field to document the
sentinel semantics and prevents generic acceptance from being mistaken for a
real player. It also requires explicit replay and wire compatibility rules.

## Review questions

The architecture maintainers must select one option, or document a narrower
versioned split, using the accepted decision, information, execution, state
closure, replay, compatibility, and API-lifecycle contracts. The review must
state whether existing fixtures containing zero are historical meanings that
must remain byte-compatible, and whether any surface needs a versioned
migration.

No option was implemented by Batch F. The numbered ADR 0050 proposal now
records the selected policy, but it is not accepted architecture until the
required independent exact-head review approves it. Until then, FND-028
remains `BLOCKED_CONTRACT_AMBIGUITY` with executable behavior unchanged; this
candidate and the proposal must not be cited as accepted normative authority.
