# Comprehensive Rules Snapshot Identity — 2026-09-15

**Status:** accepted authority identity; no implementation authorization

**Snapshot ID:** `wotc-cr-2026-08-07-txt-20260819-sha256-4381ad1b39ab2c05f7d03633a20f711ed37277074d3266dcba5f38cbb527423f`

This small repository-owned record binds the exact official artifact selected by
ADR 0051. Manafold does not redistribute the copyrighted Comprehensive Rules
document; future maintainers verify a locally acquired copy against this
record.

## Official source and variants

As of 2026-09-15, Wizards of the Coast's official [Magic rules page](https://magic.wizards.com/en/rules) identifies the current Comprehensive Rules document and exposes three official file variants: DOCX, PDF, and TXT. The document text in all three variants identifies the applicable rules as effective **August 7, 2026**. No mirror or third-party engine was used as authority.

The date in the TXT filename is `20260819`, while the embedded effective date remains `2026-08-07`. These are recorded as separate identities; the filename date is not silently substituted for the document's effective date.

### First-party page binding evidence

The selected TXT is not bound merely because an accessible
`media.wizards.com` object exists. A direct HTTP retrieval of the official
Rules page at `2026-09-15T18:13:09.3026022Z` returned HTTP 200 with
`Content-Type: text/html; charset=utf-8`, `150428` exact bytes, and SHA-256
`51aa239447a0706fa036a416284086afee62ddb703e7af11afe9375d67eba938`.
The retrieved HTML exposed these exact first-party links:

```text
DOCX = https://media.wizards.com/2026/downloads/MagicCompRules%2020260807.docx
PDF  = https://media.wizards.com/2026/downloads/MagicCompRules%2020260807.pdf
TXT  = https://media.wizards.com/2026/downloads/MagicCompRules%2020260819.txt
```

This page-level observation binds the selected TXT URL to the official source
at the time of snapshot selection. The page may later change; old conformance
evidence remains bound to this record and its TXT digest.

## Exact retrieval evidence

Each URL was fetched directly with HTTP 200 and the response bytes were written without text conversion. The HTTP `Content-Length` matched the resulting file length in every case. Retrieval timestamps are UTC and were captured immediately before each request.

| Variant | Exact official URL | Document/file name | Effective date / URL version | Content type | Retrieval timestamp (UTC) | Exact bytes | SHA-256 |
|---|---|---|---|---|---|---:|---|
| DOCX | [`MagicCompRules 20260807.docx`](https://media.wizards.com/2026/downloads/MagicCompRules%2020260807.docx) | `MagicCompRules 20260807.docx` | 2026-08-07 / `20260807` | `application/vnd.openxmlformats-officedocument.wordprocessingml.document` | `2026-09-15T17:25:44.3751843Z` | 710,355 | `4ac9238d60800af34e40f37fb478b58bc85ec6c358563048b775cc694656ed8d` |
| PDF | [`MagicCompRules 20260807.pdf`](https://media.wizards.com/2026/downloads/MagicCompRules%2020260807.pdf) | `MagicCompRules 20260807.pdf` | 2026-08-07 / `20260807` | `application/pdf` | `2026-09-15T17:25:44.7539609Z` | 2,524,708 | `9e2268a0ed58f229c5b974a3ae7986c5f91a5a052c4af1a9e672906a427c044c` |
| TXT | [`MagicCompRules 20260819.txt`](https://media.wizards.com/2026/downloads/MagicCompRules%2020260819.txt) | `MagicCompRules 20260819.txt` | 2026-08-07 / `20260819` | `text/plain` | `2026-09-15T17:25:45.1489379Z` | 977,822 | `4381ad1b39ab2c05f7d03633a20f711ed37277074d3266dcba5f38cbb527423f` |

The official page itself is the source for the variant inventory; the exact
byte lengths and SHA-256 values above are measurements of the direct downloads
at the stated timestamps. The page-level byte identity above preserves the
first-party link provenance separately from the rules-document byte identity.

## Selected authority for the first M3 rules case

Bind the **TXT** variant as Manafold's primary exact Comprehensive Rules artifact:

`https://media.wizards.com/2026/downloads/MagicCompRules%2020260819.txt`

Record its effective date as `2026-08-07` and its exact byte identity as `977822` bytes with SHA-256 `4381ad1b39ab2c05f7d03633a20f711ed37277074d3266dcba5f38cbb527423f`.

Rationale: TXT is the smallest and most direct machine-readable representation for deterministic clause extraction and citation. It avoids PDF layout interpretation and DOCX ZIP/XML packaging while remaining the exact TXT link exposed by the official page at the binding observation. Keep the PDF and DOCX identities above as official presentation/audit variants, not as separate semantic rule versions. If a case needs visual pagination evidence, use the PDF as a secondary cross-reference while retaining the TXT binding as the semantic source.

The effective rules date is **2026-08-07**. The TXT filename/version date
`20260819` and the embedded effective date are deliberately recorded as
separate facts. Conformance cases must cite this snapshot ID and the
applicable Comprehensive Rules section/rule numbers; additional official
rulings are separate authority identities when required.
