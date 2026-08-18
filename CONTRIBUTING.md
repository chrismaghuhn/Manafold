# Contributing

**Status:** accepted contribution process  
**Stability:** process


The project is in foundation phase. External contributions must not be accepted
until license and contribution terms are resolved. Internal contributors can use
this process now.

## Before changing code

1. identify the affected contract;
2. inspect ADRs and open decisions;
3. write an ADR for architectural change;
4. add the smallest failing conformance case;
5. assess schema, replay, hidden-information, and performance impact.

## Pull-request evidence

A PR should include narrow scope, authority references where applicable, happy
and illegal paths, serialization/replay evidence, information-safety review,
compatibility impact, and benchmark evidence for performance claims.

## Prohibited shortcuts

- random or first-candidate completion of player choices;
- omission of legal actions to simplify a model;
- hidden identities in debug fields, keys, errors, or ordering;
- approximate execution of unsupported Oracle text;
- unversioned replay/schema meaning changes;
- rules logic in Python, UI, transport, or reward code;
- continuing after an invariant violation.

Use the release and implementation checklists under `docs/templates/`.
