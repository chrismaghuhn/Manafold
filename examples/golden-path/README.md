# Synthetic Golden Path

**Status:** tested maintainer example; not a Magic support claim and not a certified bundle.

This directory demonstrates the maintenance path without inventing real Magic semantics:

```text
contract catalog
→ capability registry
→ card-definition manifest
→ bundle manifest
→ Decision/Response + Observation/Information/Event/PlayerStep fixtures
→ Replay fixture
→ recursive capability census
→ fail-closed certification preflight
```

The final certification must remain `blocked` because V0.2.2 contains no playable kernel or semantic runtime evidence. A certified result is a test failure.

Run:

```bash
python scripts/validate_golden_path.py
```
