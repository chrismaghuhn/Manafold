# PlayerId Zero Policy — ADR Candidate

**Status:** proposed, non-authoritative, blocks implementation of a global zero policy
**Stability:** provisional, non-authoritative
Owner: architecture maintainers
Reviewers: model/state, replay, observation, and environment maintainers
Final ADR number: not allocated

## Context

`PlayerId` is a generic canonical unsigned identifier and can currently
represent `0`. The executable system does not apply one coherent meaning to
that value: some authoritative and player-facing surfaces accept a declared
zero player, while Replay V3 explicitly rejects a zero step actor. Batch FND-028
therefore remains `BLOCKED_CONTRACT_AMBIGUITY`. This candidate records the
choice that must be reviewed; it does not change any parser, validator,
fixture, schema, replay meaning, or runtime boundary.

## Current executable matrix

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

No option is implemented by Batch F. Until an accepted, numbered ADR exists,
FND-028 is recorded as `BLOCKED_CONTRACT_AMBIGUITY` with executable behavior
unchanged. A final ADR number is allocated only after independent review and
acceptance; this candidate must not be cited as normative authority.
