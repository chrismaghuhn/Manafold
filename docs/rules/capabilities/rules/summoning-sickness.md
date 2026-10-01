# Summoning Sickness

**Capability key/version:** `rules/summoning-sickness@0.1.0`
**Lifecycle:** `covered`
**Owner role:** `combat`
**Authority snapshots:** WotC Comprehensive Rules snapshot `wotc-cr-2026-09-25-txt-20260925-sha256-8d860e451f20f38865b725b42d82feb714c725373dd8f3b32b8652b3eeb070ca`, rules 302.6, 508.1. Design: `docs/superpowers/specs/2026-10-01-vanilla-creatures-design.md` sections 3 and 4.

Covered for two-player games of admitted lands and vanilla creatures by the
listed rules and production-endpoint cases. No certification, card, deck,
format, or playability support is claimed.

## Supported scope

A creature can attack only if its controller has controlled it continuously
since their most recent turn began (CR 302.6). Only untapped creatures that can
attack are offered as attackers (CR 508.1a): the active player's creature can
attack when its record's turn is earlier than the current turn.

## Explicit exclusions

The following are not supported and fail closed:
- haste and every other keyword;
- change of control;
- permanents that become creatures later;
- the same restriction on activated abilities with a tap cost.

## State and identity model

Every battlefield permanent has a record with the turn number at which its
controller began controlling it, set when the permanent enters. Whether it can
attack is derived from that number and the controller's current turn. It is not
a stored flag: a flag would flip for every permanent at each turn start, and it
would assume nothing ever becomes a creature later. A new incarnation starts a
new record (CR 400.7).

## Events and replacement points

No event. The rule only limits which attackers are offered. There are no
replacement points.

## Decisions and ordering

The existing attacker declaration offers one `SelectObject` candidate per
creature that can attack, in the actor's opaque order. There is no cap on the
number of attackers.

## Information and opaque identities

Whether a creature can attack follows from public facts: when it entered and
whose turn it is. It is public.

## Transition/continuation behavior

The restriction is evaluated when the attacker declaration is offered, and
again when a restored or committed request is validated: the request must offer
exactly the creatures that can attack. A declaration that names a creature the
request does not offer is rejected and changes nothing. A turn on which no
creature can attack still asks, with no candidate (the domain is `ChooseMany`
from 0 to 0).

## Conformance, property, replay, and performance evidence

- Rules cases in `crates/mtgml-rules/src/turn_progression.rs`:
  `a_creature_cannot_attack_the_turn_it_arrives`,
  `a_creature_can_attack_from_its_controllers_next_turn`,
  `a_tapped_creature_is_not_offered_as_an_attacker`,
  `a_creature_is_offered_only_to_its_controller_on_their_own_turn`,
  `a_wrong_candidate_set_for_the_declaration_is_refused`.
- Production-endpoint case in `crates/mtgml-environment/tests/creature_game.rs`:
  `a_creature_cannot_attack_the_turn_it_arrives_but_can_on_its_controllers_next_turn`.
