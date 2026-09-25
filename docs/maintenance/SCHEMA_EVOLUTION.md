# Schema Evolution

**Status:** accepted schema-evolution policy  
**Stability:** normative


1. name the exact semantic surface and current version;
2. add/modify reader and writer fixtures before producer code;
3. classify compatibility and migration requirements;
4. update JSON schema, Rust codec, Python DTO, tests, and documentation together;
5. preserve old artifacts and prove reader behavior;
6. never reuse an enum/key value for new meaning;
7. publish migration provenance and content digest.

## Combat observation payload successor (2026-09-24)

The public payload `magic-m3-observation.v1` remains byte-for-byte unchanged.
Combat participation uses the explicit successor
`magic-combat-observation.v2`, bound only to the exact
`magic_combat_attackers_0_1_0` semantic execution contract. The V2 projection
exposes the defending player and combat participation with perspective-local
opaque object identities. V1 artifacts are not migrated or reinterpreted; a
V1 reader continues to reject the V2 codec as unknown.

Migration provenance: new V2 writer and Rust/Python readers are introduced
alongside the preserved V1 reader. Replay V6 admits the V2 codec only for the
new exact combat capability closure.

Schema source: `schemas/magic-combat-observation.v2.schema.json`

Content SHA-256: `232ff2962461335ceeacc4291a588134f54940dfab6871c9b9198946dc2b580a`

## Combat observation successor (2026-09-24)

The payload `magic-combat-observation.v2` remains byte-for-byte unchanged and
continues to be bound only to `magic_combat_attackers_0_1_0`. Public blocker
assignments use the successor `magic-combat-observation.v3`, bound only to the
exact `magic_combat_blockers_0_1_0` execution contract. V3 retains the same
perspective-local opaque identity shape and permits the bounded zero/one
blocker assignment. V1 and V2 readers keep their existing meanings and reject
the V3 codec as unknown.

Migration provenance: V3 adds a separate Rust/Python writer and reader, schema,
and canonical fixture. Replay V6 admits V3 only for the exact cumulative
combat + blockers capability closure; it does not reinterpret V2 artifacts.

Schema source: `schemas/magic-combat-observation.v3.schema.json`

Content SHA-256: `c51b86ef50cb024aa90e62543a92a6c6f2433e170380a21f87128ed51cf1dd6a`

## Combat observation successor (2026-09-25)

The V3 payload remains byte-for-byte unchanged and bound only to
`magic_combat_blockers_0_1_0`. Block 6 introduces V4 for the exact
`magic_combat_damage_0_1_0` identity. V4 adds both public life totals,
positive public marked damage keyed by perspective-local opaque creature ID,
explicit durable blocked/unblocked status, and public player-loss status. It
keeps the blocker relation separate, so blocked with no current blocker
remains representable. Its combat cardinality enforces the Foundation V2 bound
of one blocked attacker and at most one live blocker. V1, V2, and V3 readers
retain their meanings and reject the V4 codec as unknown.

The current official Comprehensive Rules source is
[MagicCompRules 20260925.txt](https://media.wizards.com/2026/downloads/MagicCompRules%2020260925.txt),
effective 2026-09-25, SHA-256
`8d860e451f20f38865b725b42d82feb714c725373dd8f3b32b8652b3eeb070ca`.
The official [Rules page](https://magic.wizards.com/en/rules) linked to that
exact TXT artifact during the independent review correction. The downloaded
artifact was 977,752 bytes; its stated effective date and measured SHA-256
matched the linked identity above.
The page's resolved TXT URL was
`https://media.wizards.com/2026/downloads/MagicCompRules%2020260925.txt`.
Compared with the accepted Foundation V2 snapshot
`wotc-cr-2026-08-07-txt-20260819-sha256-4381ad1b39ab2c05f7d03633a20f711ed37277074d3266dcba5f38cbb527423f`,
the reviewed clauses have zero text differences. The scope comparison is
`MATERIAL_FOUNDATION_DISCREPANCY = NO`; the implementation continues to use
Foundation V2's bounded capability scope and pins the current CR identity in
the new cumulative semantic contract.

Clauses compared: CR 119.2–119.3; 120.1, 120.2a, 120.3a, 120.3e,
120.4b–120.4d, 120.5–120.6, 120.8; 506.4; 509.1g–509.1h, 509.2; 510.1,
510.1a–510.1d, 510.2–510.3; 703.4k, 703.4m; 704.1–704.3, 704.5a, 704.5f,
704.5g, 704.8; and 701.8.

Schema source: `schemas/magic-combat-observation.v4.schema.json`.

Content SHA-256: `b18493bc4dbeb9ea0dffcc3e33c2e655df952d25b094abbd80d885ae92b97a37`
