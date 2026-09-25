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
