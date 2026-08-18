# Card Content Directory Layout

**Status:** accepted maintainer convention

```text
cards/
├── capabilities/
│   ├── registry.json
│   ├── registry.example.json
│   └── specs/
├── definitions/
│   └── <definition-id path>/
│       ├── manifest.json
│       ├── README.md
│       └── cases/
├── generated/
│   └── <generator identity>/<source digest>/...
├── decks/
│   └── <deck-id>.json
└── bundles/
    └── <bundle-id>/
        ├── manifest.json
        ├── certification.json
        └── evidence/
```

Definition IDs map to directories without lossy renaming. Example: `project/card/example-card` maps to `cards/definitions/project/card/example-card/`.

Generated content cannot be imported by production card registries directly. Promotion copies/re-expresses reviewed semantics under `definitions/` and preserves provenance.

Evidence files should be machine-readable when possible and refer to immutable case/result IDs rather than local absolute paths.
