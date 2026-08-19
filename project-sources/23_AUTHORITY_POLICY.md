# Rules Authority Policy

**Status:** accepted process  
**Stability:** normative maintenance policy

## Separate immutable authorities

Every certified claim pins distinct identities for:

1. Comprehensive Rules;
2. Oracle/card source;
3. official card rulings used by the scope;
4. Commander/format policy;
5. banlist/deck-legality policy;
6. project interpretation records for unresolved implementation questions.

## Precedence

Within a scoped conformance case, maintainers document the applicable text and why it controls. Oracle text and official rulings are interpreted under the pinned Comprehensive Rules. Format policy may add format semantics but cannot silently rewrite core rules.

## Independent engines

Forge, XMage, Argentum, or another engine may provide differential evidence. They are not final authority. Disagreement triggers analysis and a pinned case; it is not resolved by majority vote.

## Project interpretations

When authority is ambiguous for implementation, create an interpretation record containing:

- exact snapshots and relevant clauses;
- competing readings;
- chosen behavior and rationale;
- affected capabilities/cards;
- conformance cases;
- review owner and supersession rule.

No informal chat conclusion becomes engine behavior without this evidence.
