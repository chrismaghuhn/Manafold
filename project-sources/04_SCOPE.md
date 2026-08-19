# Scope

**Status:** product boundary accepted; executable content blocked  
**Stability:** normative V1 boundary  
**Last reviewed:** 2026-08-18

## V1 environment

A local, headless, deterministic environment for **two-player official Commander using two exact fixed decks**.

Accepted properties:

- exactly two players and no sideboards;
- immutable rules, format-policy, legality, Oracle/source, rulings, bundle, schema, and digest identities;
- every player-relevant choice exposed through the decision protocol;
- one auditable reference backend on one host/process baseline;
- Rust semantic engine plus rules-free Python client/ML tooling;
- technical truncation only as a safety outcome, never a rules draw;
- no human UI requirement.

## Executable closure

Scope includes not only listed cards but every reachable dependency:

- card faces and characteristics;
- tokens, copies, emblems, named/generated objects;
- costs, targets, modes, values, ordering, and payments;
- triggers, replacement/prevention effects, continuous/copy semantics;
- counters, zones, identity, Last Known Information, stack, combat;
- Commander designation, tax, damage, and zone decisions;
- choices delegated to either player;
- loops and technical safety boundaries;
- knowledge/opaque-ID behavior and every observed-event variant.

## V1 exclusions

Four-player Commander, arbitrary decks, general legality service, draft/sealed, sideboarding, UI/accounts, network servers, cluster training, authoritative direct Oracle execution, a second optimized backend, and a strong learned agent are excluded.

## Scope lock

Card certification starts only after M2.5 commits:

- exact deck and generated-object manifests;
- authority/source snapshots;
- recursive capability census;
- explicit unsupported/excluded cases;
- reference hardware and numerical acceptance thresholds;
- scope-impact and certification manifests.

Replacing one card or snapshot creates a new bundle identity and impact report.
