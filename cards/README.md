# Card Content

This tree contains project-authored manifests and future executable card semantics. It contains no copyrighted card images and no bundled third-party card corpus.

```text
capabilities/  versioned registry, specs, and evidence references
definitions/   reviewed authored card definitions and manifests
generated/     reproducible intermediate candidates; never authoritative
bundles/       immutable bundle manifests and certification evidence
decks/         exact immutable deck manifests
```

## Certification vocabulary

```text
Imported -> Parsed -> Implemented -> Covered -> Certified
```

Only a locked bundle can confer `Certified` status. Parser success rate, compilation, file count, or game completion is not a support claim.

## Maintainer commands

```bash
python scripts/scaffold_card.py project/card/example-card "Example Card"
python scripts/scaffold_capability.py mechanic/example "Example Mechanic"
python scripts/capability_census.py \
  --bundle cards/bundles/example-v1/manifest.example.json \
  --registry cards/capabilities/registry.example.json \
  --minimum-capability-lifecycle proposed \
  --minimum-card-lifecycle draft
python scripts/certify_bundle.py \
  --bundle cards/bundles/example-v1/manifest.example.json \
  --registry cards/capabilities/registry.example.json \
  --output /tmp/certification.json  # exits 2 because the example is intentionally uncertified
python scripts/validate_maintainer_artifacts.py
```

See [`../docs/cards/ADDING_CARDS.md`](../docs/cards/ADDING_CARDS.md) and [`../docs/cards/CAPABILITY_MODEL.md`](../docs/cards/CAPABILITY_MODEL.md).
