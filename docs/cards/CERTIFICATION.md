# Card and Bundle Certification

**Status:** accepted policy  
**Stability:** normative support-claim contract

## Certification unit

The authoritative support claim is a **locked capability bundle**, not a global card count and not an isolated card file.

A certified bundle identifies:

- engine/build and backend;
- rules, format-policy, Oracle/source, and ruling snapshots;
- exact deck manifests;
- exact card definitions and generated/reference objects;
- recursive capability closure and versions;
- schemas and digest algorithms;
- conformance, soundness, completeness, information, replay, fuzz/soak, and performance evidence;
- explicit exclusions and known limitations.

## Card lifecycle versus bundle certification

A card can be `Implemented` or `Covered` in development. It becomes `Certified` only because a particular immutable bundle containing it passes all gates. The same card under a different snapshot or dependency version requires new certification evidence.

## Static preflight

```bash
python scripts/certify_bundle.py --bundle cards/bundles/<bundle>/manifest.json --output report.json
```

The script is conservative. It can prove missing manifests, capabilities, dependencies, statuses, or evidence references. It cannot by itself prove Magic correctness; runtime gate artifacts are still required.

## Revocation

A discovered semantic, information, replay, or determinism defect marks affected certifications `revoked` or `superseded`. Published artifacts remain available with the defect notice and exact provenance; they are not silently replaced.

## Claim language

Allowed:

> Bundle `X` is certified for engine build `Y` under snapshots `R/F/O`, with the exclusions listed in report `Z`.

Not allowed:

> The engine supports every parsed card.

> This card compiles, so it is supported.

> The game usually finishes, so the matchup is complete.

A certification report records the complete capability closure and every required gate. The closure is typed rather than flattened into diagnostic strings: dependency cycles retain their full paths; lifecycle blockers retain both the artifact key and current lifecycle; missing definitions and quarantined native executors remain explicit blockers.

## Native-executor closure gate

Before any other native-executor evidence is considered, certification traverses actual definition closure. The automatically discovered executor set must exactly equal the bundle declaration. Discovered, undeclared, and stale executor sets are preserved in the certification report; any nonempty set blocks certification under the current quarantine policy.
