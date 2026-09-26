# `basic-land@1.0.0` content record

This project-authored content catalog contains the closed Mountain and Plains
definitions used by the first profile. The canonical manifest bytes are
identical to the pre-existing Phase-2 ContentContract V1 KAT:

```text
content_contract_id = 80d26c187739664e880948e767e44ed9791aa7c25e7ef703e6d63385311fb346
content manifest    = content-contract.v1.cbor
provenance catalog  = provenance.v1.cbor
```

## Pinned source evidence

The source is Scryfall's Oracle Cards bulk snapshot updated at
`2026-09-25T21:01:58.069Z`:

```text
snapshot id = oracle-cards-20260925210158
URI         = https://data.scryfall.io/oracle-cards/oracle-cards-20260925210158.jsonl.gz
archive SHA-256 = c607300fe03ce0d9f59181b1bb33d8001e2fa339b8c68eefd70a6501fa757623
```

`provenance.v1.cbor` stores the Oracle UUID (`oracle_id`), source-record codec
identity, and SHA-256 of each exact decompressed JSONL record byte sequence.
The record digest includes the terminating LF and is computed over the bytes
as stored in the pinned archive, without JSON parsing or reserialization.
The bulk records themselves are not redistributed in this repository.

| Definition ID | Profile subtype | Oracle UUID | Exact JSONL record SHA-256 |
| --- | --- | --- | --- |
| `1` | Mountain | `a3fb7228-e76b-4e96-a40e-20b5fed75685` | `b57d8ce5dcbbb01e1c128aaa9ebeab8a26d5f297edb51eac6adce5c4af770033` |
| `2` | Plains | `bc71ebf6-2056-41f7-be35-b2e5c34afa99` | `af82e883368b8211c1845af680e1b4dab52666b41969cc6987bffdde7ada86b7` |

The exact retrieved archive SHA-256 was checked before extracting these
records. The pinned semantic source identifies the definitions; subtype
semantics come from CR 305.6, not card-name dispatch or reminder text.

## Admission boundary

This catalog and source provenance do not themselves authorize execution.
Profile admission additionally verifies the complete recursively derived
capability closure and its Rules/Semantic/Execution identity chain. The
admission token is for later RulesKernel integration; it does not activate a
runtime, implement gameplay, advance capability lifecycle beyond `specified`,
or claim coverage/certification.
