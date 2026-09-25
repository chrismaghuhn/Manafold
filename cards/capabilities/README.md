# Capability Registry

`registry.json` is the project capability registry. On the M3 final-closure
candidate branch, all eleven Foundation V2 capabilities are recorded as
`covered` for their exact bounded scopes, with implementation paths and
executable conformance cases. This lifecycle reconciliation is pending
independent exact-head review and hosted CI; it does not claim certification.

The closure evidence and the 16 M3 interaction obligations are in
[`docs/reviews/m3-final-closure-2026-09-25.md`](../../docs/reviews/m3-final-closure-2026-09-25.md).
No capability, card, deck, bundle, format, Commander, broad Magic, or
playability support is certified or claimed. `registry.example.json`
demonstrates the schema without claiming support.

Use `python scripts/scaffold_capability.py <key> <title>` to add a proposal.
Lifecycle and certification rules are defined in
[`docs/cards/CAPABILITY_MODEL.md`](../../docs/cards/CAPABILITY_MODEL.md).
