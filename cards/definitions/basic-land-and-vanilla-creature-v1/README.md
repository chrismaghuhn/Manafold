# Basic lands and vanilla creatures content record

This project-authored content catalog holds five closed definitions: the
Mountain and Plains of `basic-land@1.0.0`, and three creatures of
`vanilla-creature@1.0.0`. Definitions 1 and 2 are byte-for-byte the definitions
of `cards/definitions/basic-land-v1/`, which stays as it is for land-only games.

```text
content_contract_id = 9d6ae69521530459f172c4620ed22a096a866f5351ce0f2383edaa6d2464b06d
content manifest    = content-contract.v1.cbor
provenance catalog  = provenance.v1.cbor
known answer        = persistence/golden/content-contract-basic-land-and-vanilla-creature-v1-kat.v1.json
```

## Pinned source evidence

The source is Scryfall's Oracle Cards bulk snapshot updated at
`2026-09-25T21:01:58.069Z`:

```text
snapshot id = oracle-cards-20260925210158
URI         = https://data.scryfall.io/oracle-cards/oracle-cards-20260925210158.jsonl.gz
archive SHA-256 = c607300fe03ce0d9f59181b1bb33d8001e2fa339b8c68eefd70a6501fa757623
```

`provenance.v1.cbor` stores, for each definition, the Oracle UUID (`oracle_id`),
the source-record codec identity (`scryfall.oracle-card-jsonl-record.v1`), and
the SHA-256 of the exact decompressed JSONL record byte sequence. The record
digest includes the terminating LF and is computed over the bytes as stored in
the pinned archive, without JSON parsing or reserialization. The bulk records
themselves are not redistributed in this repository.

| Definition ID | Card | Profile | Oracle UUID | Exact JSONL record SHA-256 |
| --- | --- | --- | --- | --- |
| `1` | Mountain | `basic-land@1.0.0` | `a3fb7228-e76b-4e96-a40e-20b5fed75685` | `b57d8ce5dcbbb01e1c128aaa9ebeab8a26d5f297edb51eac6adce5c4af770033` |
| `2` | Plains | `basic-land@1.0.0` | `bc71ebf6-2056-41f7-be35-b2e5c34afa99` | `af82e883368b8211c1845af680e1b4dab52666b41969cc6987bffdde7ada86b7` |
| `3` | Savannah Lions | `vanilla-creature@1.0.0` | `60ba93eb-39e6-4af2-9c66-cd38f72daff2` | `84fce7b698d07816432dc2674941ffdc9e0ffe586793ad669310f1bbd438c4cf` |
| `4` | Gray Ogre | `vanilla-creature@1.0.0` | `83c8a3a6-2e1a-4e26-8847-6d066f42d906` | `f07e4de80207bf5701840eb63bc8f35070e2e07dca93721d4fabb03c16fc79fa` |
| `5` | Hill Giant | `vanilla-creature@1.0.0` | `342199e0-15b6-4824-83da-25caef2592b3` | `7278c29c3e7fe48fd5251930df1df553c5773e2199738a183d895ec3a0fc1952` |

The creatures' characteristics, which admission compares with the definition:

| Card | Mana cost | Type line | Power/toughness |
| --- | --- | --- | --- |
| Savannah Lions | `{W}` | Creature — Cat | 2/1 |
| Gray Ogre | `{2}{R}` | Creature — Ogre | 2/2 |
| Hill Giant | `{3}{R}` | Creature — Giant | 3/3 |

All three are vanilla: `layout` normal, no `card_faces`, empty `oracle_text`
and `keywords`, integer power and toughness with toughness at least 1. The
records were extracted from the pinned archive, after its SHA-256 was checked,
with `scripts/extract_oracle_records.py --vanilla-creature`, which refuses a
record that is not a vanilla creature. The manifest was written from that output
by the `writes_the_combined_catalog` test in
`crates/mtgml-card-ir/tests/combined_catalog.rs`, run once with `--ignored`.
The known-answer file was derived separately, with the Python encoder, and the
Rust encoder, decoder and identity gate must reproduce it.

## Admission boundary

Admission (`crates/mtgml-card-ir/src/preflight.rs`) binds each definition to
exactly one pinned record. It compares the snapshot, codec, Oracle UUID, record
digest, profile and characteristics, so a manifest cannot pair a pinned record
with other characteristics. A record can back at most one definition.

A vanilla creature derives the roots `rules/cast-creature-spell`,
`rules/stack-resolution`, `rules/summoning-sickness`, `rules/combat-damage`,
`rules/damage-and-life` and `rules/state-based-actions-combat`. They are roots
of the creature profile, not of every game: admitting only the lands keeps the
land closure and its rules contract identity.

This catalog does not itself authorize execution or claim that any capability
is implemented. The capabilities above are `specified`.
