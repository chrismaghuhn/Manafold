# ContextApplicationV2 Slice 3 Semantic Validator Implementation Plan

**Status:** provisional
**Stability:** provisional

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement the Slice-3 semantic validator that proves exact V1 theorem, candidate/source-instance, historical source-context, V2 bridge, precondition, and source-bound evidence equivalence without implementing V3 acceptance.

**Architecture:** Keep the accepted V2 DTOs, schemas, identity registry, and Slice-2 closure algebra unchanged. Add a pure semantic core with identical Python and Rust behavior, a narrow reusable V1 source/member seam, and a Python integration validator that composes exact base-authority, candidate/source, and evidence resolution around that core.

**Tech Stack:** Python 3.11-3.13, frozen dataclasses, existing canonical-CBOR and digest-envelope primitives, `unittest`, existing `AuthorityValidator`, `AuthoritySourceResolver`, `ContextApplicationV2Resolver`, Rust 2021, `serde_json`, and the existing repository verification scripts.

---

## Starting state and invariants

Implementation starts from branch `chris/context-application-v2-slice3-semantic-validator` at the implementation base `9a42c3424e61e54565ff0cd17b3f2f2780f78562`, plus the already committed design documents `63f059c`, `b8c585e`, and `6fc43c4`. The untracked plan in the original checkout is outside this worktree and must remain untouched.

The following contracts must remain true throughout the plan:

```text
SourceInstanceV1.source_context
    == member.bridge.context[*].source_value

ContextProofV1.context_dimensions
    == member.bridge.context[*].reviewed_value

ContextProofV1.temporal_semantics
    == member.bridge.temporal[*].reviewed_value

source_value == reviewed_value
    -> exact_match

source_value != reviewed_value
    -> reviewed_divergence
```

The V1 `source_context` precondition continues to compare with the historical
SourceInstanceV1 value. `temporal_semantic` keeps only its existing V1
theorem/attestation behavior. `class_projection` keeps its existing
fail-closed requirement for `member_proof_attestation`; the V2 member shape
does not gain that field.

## File map

| File | Responsibility in this plan |
| --- | --- |
| `scripts/context_application_v2_validator.py` | Pure Python semantic input/result/error types, pure bridge validator, evidence-ownership checks, and filesystem-aware V2 composition. |
| `scripts/authority_validator.py` | Narrow reusable V1 context-member source/precondition seam and read-only validated-record lookup. |
| `scripts/context_application_v2_resolver.py` | Thin typed member-to-candidate/source-instance helper; no semantic rules or second filesystem resolver. |
| `python/tests/test_context_application_v2_validator.py` | Python pure-core matrix tests, synthetic integration fixtures, identity checks, evidence checks, negative matrix, and repeatability. |
| `conformance/fixtures/authority/context_application_v2_semantic_golden_matrix.v1.json` | Shared pure-semantic input and expected result/error categories for Python and Rust. |
| `crates/mtgml-persistence/src/authority.rs` | Rust pure semantic input/error types and exact bridge/precondition comparison function. |
| `crates/mtgml-persistence/src/tests.rs` | Rust execution of the shared semantic matrix and direct relation/precondition controls. |
| `scripts/run_python_tests.py` | Add the new validator test module to the explicit smoke profile. |
| `scripts/verify_repository.py` | Require the new semantic golden fixture as a repository contract artifact. |
| `docs/normative-document-register.v1.json` | Register this implementation plan so the documentation gate sees it. |

Do not modify `schemas/context-application-authority.v2.schema.json`, any
identity or closure golden fixture, `docs/adr/0042-context-application-v2-reviewed-context-bridge.md`, production authority files, or current C outputs.

---

### Task 1: Add the shared pure-semantic matrix and Python RED tests

**Files:**
- Create: `conformance/fixtures/authority/context_application_v2_semantic_golden_matrix.v1.json`
- Create: `python/tests/test_context_application_v2_validator.py`

- [ ] **Step 1: Add the failing Python matrix tests.**

Create `python/tests/test_context_application_v2_validator.py` with tests that
load the new matrix, construct the not-yet-existing
`ContextApplicationV2SemanticInput`, and call
`validate_context_application_v2_semantics`. The first run must fail because
the production module is missing.

```python
from context_application_v2_validator import (
    ContextApplicationV2SemanticInput,
    ContextApplicationV2SemanticValidationError,
    ContextPreconditionValueV1,
    validate_context_application_v2_semantics,
)


class ContextApplicationV2SemanticCoreTests(unittest.TestCase):
    def test_exact_match_control_is_accepted(self) -> None:
        result = validate_context_application_v2_semantics(self._case("exact_match"))
        self.assertTrue(result.valid)
        self.assertIsNone(result.error_code)

    def test_reviewed_divergence_control_is_accepted_without_v1_source_equality(self) -> None:
        result = validate_context_application_v2_semantics(
            self._case("reviewed_divergence")
        )
        self.assertTrue(result.valid)
        self.assertIsNone(result.error_code)

    def test_every_negative_matrix_case_returns_its_declared_error_code(self) -> None:
        for raw in self._matrix()["cases"]:
            if raw["expected"]["valid"]:
                continue
            with self.subTest(case_id=raw["case_id"]):
                with self.assertRaises(ContextApplicationV2SemanticValidationError) as error:
                    validate_context_application_v2_semantics(self._case(raw["case_id"]))
                self.assertEqual(error.exception.code, raw["expected"]["error_code"])
```

`_case` must convert the matrix's relation strings to
`ContextBridgeRelationV2` and convert theorem `payload` and member
`observed_value` entries to `ContextPreconditionValueV1` without reordering.
Import `cast` from `typing` and `PersistenceValue` from `mtgml.persistence`.
Use these concrete read-only helpers:

~~~python
def _matrix(self) -> dict[str, object]:
    return cast(
        dict[str, object],
        json.loads(
            (
                ROOT
                / "conformance/fixtures/authority/"
                / "context_application_v2_semantic_golden_matrix.v1.json"
            ).read_text(encoding="utf-8")
        ),
    )


def _case(self, case_id: str) -> ContextApplicationV2SemanticInput:
    cases = cast(list[dict[str, object]], self._matrix()["cases"])
    raw = next(case for case in cases if case["case_id"] == case_id)
    return ContextApplicationV2SemanticInput(
        theorem_subject_shape=cast(PersistenceValue, raw["theorem_subject_shape"]),
        member_context_binding=cast(PersistenceValue, raw["member_context_binding"]),
        historical_source_values=tuple(cast(list[str], raw["historical_source_values"])),
        bridge_source_values=tuple(cast(list[str], raw["bridge_source_values"])),
        theorem_context_values=tuple(cast(list[str], raw["theorem_context_values"])),
        bridge_reviewed_values=tuple(cast(list[str], raw["bridge_reviewed_values"])),
        bridge_relations=tuple(
            ContextBridgeRelationV2(value)
            for value in cast(list[str], raw["bridge_relations"])
        ),
        theorem_temporal_values=tuple(cast(list[str], raw["theorem_temporal_values"])),
        bridge_temporal_values=tuple(cast(list[str], raw["bridge_temporal_values"])),
        theorem_preconditions=tuple(
            ContextPreconditionValueV1(
                value["precondition_id"],
                cast(PersistenceValue, value["payload"]),
            )
            for value in cast(list[dict[str, object]], raw["theorem_preconditions"])
        ),
        member_preconditions=tuple(
            ContextPreconditionValueV1(
                value["precondition_id"],
                cast(PersistenceValue, value["observed_value"]),
            )
            for value in cast(list[dict[str, object]], raw["member_preconditions"])
        ),
    )
~~~

- [ ] **Step 2: Add the complete shared matrix.**

Create the fixture with schema
`manafold.m2.5.c.context-application-v2-semantic-golden-matrix.v1` and one
fully specified case for each row below. Every case contains the exact ten
context values, ten relations, four temporal values, and both precondition
vectors; only the named field differs from the exact-match control.

| Case ID | Mutation | Expected result |
| --- | --- | --- |
| `exact_match` | all historical, source, theorem, and reviewed context values are `not_applicable`; all relations are `exact_match` | valid |
| `reviewed_divergence` | context slot `timing`: historical/source `not_applicable`, theorem/reviewed `activation_time`, relation `reviewed_divergence` | valid |
| `member_shape_mismatch` | member host relationship `same_host`, theorem host relationship `cross_host` | `MEMBER_SUBJECT_MISMATCH` |
| `source_value_mismatch` | bridge source `timing` is `battlefield`, historical source is `not_applicable` | `MEMBER_SOURCE_CONTEXT_MISMATCH` |
| `reviewed_context_mismatch` | bridge reviewed `timing` is `activation_time`, theorem value is `not_applicable` | `MEMBER_REVIEWED_CONTEXT_MISMATCH` |
| `reviewed_temporal_mismatch` | bridge `trigger_order` is `immediate`, theorem value is `not_applicable` | `MEMBER_TEMPORAL_MISMATCH` |
| `relation_mismatch` | source/reviewed timing differ but relation is `exact_match` | `BRIDGE_RELATION_MISMATCH` |
| `precondition_coverage_mismatch` | theorem has `p0=["zone", "not_applicable"]`, member has no attestation | `PRECONDITION_COVERAGE` |
| `precondition_id_mismatch` | theorem ID `p0`, member ID `p1`, equal payloads | `PRECONDITION_MISMATCH` |
| `precondition_value_mismatch` | theorem `p0=["zone", "not_applicable"]`, member observed `p0=["zone", "battlefield"]` | `PRECONDITION_MISMATCH` |

Use this exact subject shape in every case:

```json
["binary", "directed", [[0, "source", "card", "card.a"], [1, "affected", "card", "card.b"]], "cross_host"]
```

Use the same value order as `CONTEXT_DIMENSIONS` and `TEMPORAL_SEMANTICS` in
`python/src/mtgml/authority.py`. Do not add evidence or source-resolution
fields to this pure fixture.

- [ ] **Step 3: Run the Python RED tests.**

```powershell
python -m unittest python.tests.test_context_application_v2_validator.ContextApplicationV2SemanticCoreTests -v
```

Expected: `ERROR` because `scripts/context_application_v2_validator.py` has
not been created. Do not add a placeholder module before recording this
failure.

---

### Task 2: Implement the pure Python semantic core

**Files:**
- Create: `scripts/context_application_v2_validator.py`
- Test: `python/tests/test_context_application_v2_validator.py`

- [ ] **Step 1: Add the immutable semantic types.**

Create the module with the same `python/src` import setup used by the other
`scripts` modules. Define these types exactly:

~~~python
from dataclasses import dataclass
from typing import Final, cast

from mtgml.authority import ContextBridgeRelationV2
from mtgml.persistence import PersistenceValue

CONTEXT_SLOT_COUNT: Final = 10
TEMPORAL_SLOT_COUNT: Final = 4
CONTEXT_DIMENSIONS: Final = (
    "zone",
    "visibility",
    "timing",
    "temporal_order",
    "source_affected_relation",
    "control_ownership_relation",
    "replacement_layer_relation",
    "trigger_lki_relation",
    "information_relation",
    "decision_actor_relation",
)
TEMPORAL_SEMANTICS: Final = (
    "trigger_order",
    "dependency_order",
    "duration",
    "replacement_order",
)


@dataclass(frozen=True)
class ContextPreconditionValueV1:
    precondition_id: str
    value: PersistenceValue


class ContextApplicationV2SemanticValidationError(ValueError):
    def __init__(self, code: str, location: str) -> None:
        self.code = code
        self.location = location
        super().__init__(f"{code} at {location}")


@dataclass(frozen=True)
class ContextApplicationV2SemanticValidationResult:
    valid: bool
    error_code: str | None = None


@dataclass(frozen=True)
class ContextApplicationV2SemanticInput:
    theorem_subject_shape: PersistenceValue
    member_context_binding: PersistenceValue
    historical_source_values: tuple[str, ...]
    bridge_source_values: tuple[str, ...]
    theorem_context_values: tuple[str, ...]
    bridge_reviewed_values: tuple[str, ...]
    bridge_relations: tuple[ContextBridgeRelationV2, ...]
    theorem_temporal_values: tuple[str, ...]
    bridge_temporal_values: tuple[str, ...]
    theorem_preconditions: tuple[ContextPreconditionValueV1, ...]
    member_preconditions: tuple[ContextPreconditionValueV1, ...]
~~~

The module's `__all__` must expose the two input/result types, the precondition
type, the exception, the integration validator added later in this plan, and
the pure validation function.

- [ ] **Step 2: Implement exact pure comparisons.**

Implement `validate_context_application_v2_semantics` with no normalization,
fallback, lexical inference, evidence access, or SourceInstance temporal
lookup. It must perform these checks in order:

~~~python
def validate_context_application_v2_semantics(
    value: ContextApplicationV2SemanticInput,
) -> ContextApplicationV2SemanticValidationResult:
    if value.member_context_binding != value.theorem_subject_shape:
        raise ContextApplicationV2SemanticValidationError(
            "MEMBER_SUBJECT_MISMATCH", "member_context_binding"
        )
    if len(value.historical_source_values) != CONTEXT_SLOT_COUNT:
        raise ContextApplicationV2SemanticValidationError(
            "MEMBER_SOURCE_CONTEXT_MISMATCH", "historical_source_values"
        )
    if len(value.bridge_source_values) != CONTEXT_SLOT_COUNT:
        raise ContextApplicationV2SemanticValidationError(
            "MEMBER_SOURCE_CONTEXT_MISMATCH", "bridge_source_values"
        )
    if len(value.theorem_context_values) != CONTEXT_SLOT_COUNT:
        raise ContextApplicationV2SemanticValidationError(
            "MEMBER_REVIEWED_CONTEXT_MISMATCH", "theorem_context_values"
        )
    if len(value.bridge_reviewed_values) != CONTEXT_SLOT_COUNT:
        raise ContextApplicationV2SemanticValidationError(
            "MEMBER_REVIEWED_CONTEXT_MISMATCH", "bridge_reviewed_values"
        )
    if len(value.bridge_relations) != CONTEXT_SLOT_COUNT:
        raise ContextApplicationV2SemanticValidationError(
            "BRIDGE_RELATION_MISMATCH", "bridge_relations"
        )
    for index, (historical, source) in enumerate(
        zip(value.historical_source_values, value.bridge_source_values, strict=True)
    ):
        if historical != source:
            raise ContextApplicationV2SemanticValidationError(
                "MEMBER_SOURCE_CONTEXT_MISMATCH", f"context[{index}]"
            )
    for index, (theorem, reviewed) in enumerate(
        zip(value.theorem_context_values, value.bridge_reviewed_values, strict=True)
    ):
        if theorem != reviewed:
            raise ContextApplicationV2SemanticValidationError(
                "MEMBER_REVIEWED_CONTEXT_MISMATCH", f"context[{index}]"
            )
    for index, (source, reviewed, relation) in enumerate(
        zip(
            value.bridge_source_values,
            value.bridge_reviewed_values,
            value.bridge_relations,
            strict=True,
        )
    ):
        expected = (
            ContextBridgeRelationV2.EXACT_MATCH
            if source == reviewed
            else ContextBridgeRelationV2.REVIEWED_DIVERGENCE
        )
        if relation is not expected:
            raise ContextApplicationV2SemanticValidationError(
                "BRIDGE_RELATION_MISMATCH", f"context[{index}]"
            )
    if len(value.theorem_temporal_values) != TEMPORAL_SLOT_COUNT:
        raise ContextApplicationV2SemanticValidationError(
            "MEMBER_TEMPORAL_MISMATCH", "theorem_temporal_values"
        )
    if len(value.bridge_temporal_values) != TEMPORAL_SLOT_COUNT:
        raise ContextApplicationV2SemanticValidationError(
            "MEMBER_TEMPORAL_MISMATCH", "bridge_temporal_values"
        )
    for index, (theorem, reviewed) in enumerate(
        zip(value.theorem_temporal_values, value.bridge_temporal_values, strict=True)
    ):
        if theorem != reviewed:
            raise ContextApplicationV2SemanticValidationError(
                "MEMBER_TEMPORAL_MISMATCH", f"temporal[{index}]"
            )
    if len(value.theorem_preconditions) != len(value.member_preconditions):
        raise ContextApplicationV2SemanticValidationError(
            "PRECONDITION_COVERAGE", "preconditions"
        )
    for index, (theorem, member) in enumerate(
        zip(value.theorem_preconditions, value.member_preconditions, strict=True)
    ):
        if (
            theorem.precondition_id != member.precondition_id
            or theorem.value != member.value
        ):
            raise ContextApplicationV2SemanticValidationError(
                "PRECONDITION_MISMATCH", f"preconditions[{index}]"
            )
    return ContextApplicationV2SemanticValidationResult(valid=True)
~~~

- [ ] **Step 3: Run the Python pure-core tests.**

~~~powershell
python -m unittest python.tests.test_context_application_v2_validator.ContextApplicationV2SemanticCoreTests -v
~~~

Expected: exact-match and reviewed-divergence pass, every negative matrix case
reports its declared error code, and no test uses a mock resolver or source
text.

- [ ] **Step 4: Commit the pure Python core.**

~~~powershell
git add scripts/context_application_v2_validator.py python/tests/test_context_application_v2_validator.py conformance/fixtures/authority/context_application_v2_semantic_golden_matrix.v1.json
git commit -m "feat: add ContextApplicationV2 pure semantic core"
~~~

---

### Task 3: Implement the Rust pure semantic core and parity test

**Files:**
- Modify: `crates/mtgml-persistence/src/authority.rs` after `ContextBridgeRelationV2`
- Modify: `crates/mtgml-persistence/src/tests.rs` after the existing V2 identity tests
- Test: `conformance/fixtures/authority/context_application_v2_semantic_golden_matrix.v1.json`

- [ ] **Step 1: Add the failing Rust matrix test.**

Add `context_application_v2_semantic_golden_matrix_matches_python_contract`.
Load the fixture through `include_str!`, convert JSON null, booleans, signed
integers, strings, and arrays into `cbor::Value`, construct the not-yet-existing
Rust semantic input, and compare `Ok(())` or `error.code` to the fixture's
`expected` object. Reject JSON objects in the test conversion helper.

Use this exact relation conversion:

~~~rust
fn relation(value: &str) -> authority::ContextBridgeRelationV2 {
    match value {
        "exact_match" => authority::ContextBridgeRelationV2::ExactMatch,
        "reviewed_divergence" => authority::ContextBridgeRelationV2::ReviewedDivergence,
        other => panic!("unknown relation {other}"),
    }
}
~~~

Run:

~~~powershell
cargo test -p mtgml-persistence context_application_v2_semantic_golden_matrix_matches_python_contract --locked
~~~

Expected: compile failure because the Rust semantic types and function do not
exist.

- [ ] **Step 2: Add exact Rust semantic types.**

Add these public types to `crates/mtgml-persistence/src/authority.rs`:

~~~rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContextPreconditionValueV1 {
    pub precondition_id: String,
    pub value: cbor::Value,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContextApplicationV2SemanticInput {
    pub theorem_subject_shape: cbor::Value,
    pub member_context_binding: cbor::Value,
    pub historical_source_values: Vec<String>,
    pub bridge_source_values: Vec<String>,
    pub theorem_context_values: Vec<String>,
    pub bridge_reviewed_values: Vec<String>,
    pub bridge_relations: Vec<ContextBridgeRelationV2>,
    pub theorem_temporal_values: Vec<String>,
    pub bridge_temporal_values: Vec<String>,
    pub theorem_preconditions: Vec<ContextPreconditionValueV1>,
    pub member_preconditions: Vec<ContextPreconditionValueV1>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContextApplicationV2SemanticError {
    pub code: &'static str,
    pub location: String,
}
~~~

- [ ] **Step 3: Add the Rust exact-comparison function.**

Implement:

~~~rust
pub fn validate_context_application_v2_semantics(
    input: &ContextApplicationV2SemanticInput,
) -> Result<(), ContextApplicationV2SemanticError>
~~~

Implement the function with this body. Use the existing private
`CONTEXT_DIMENSIONS` and `TEMPORAL_SEMANTICS` arrays for length checks:

~~~rust
pub fn validate_context_application_v2_semantics(
    input: &ContextApplicationV2SemanticInput,
) -> Result<(), ContextApplicationV2SemanticError> {
    let error = |code: &'static str, location: &str| ContextApplicationV2SemanticError {
        code,
        location: location.to_owned(),
    };
    if input.member_context_binding != input.theorem_subject_shape {
        return Err(error("MEMBER_SUBJECT_MISMATCH", "member_context_binding"));
    }
    if input.historical_source_values.len() != CONTEXT_DIMENSIONS.len()
        || input.bridge_source_values.len() != CONTEXT_DIMENSIONS.len()
    {
        return Err(error("MEMBER_SOURCE_CONTEXT_MISMATCH", "context_values"));
    }
    if input.theorem_context_values.len() != CONTEXT_DIMENSIONS.len()
        || input.bridge_reviewed_values.len() != CONTEXT_DIMENSIONS.len()
    {
        return Err(error("MEMBER_REVIEWED_CONTEXT_MISMATCH", "context_bridge"));
    }
    if input.bridge_relations.len() != CONTEXT_DIMENSIONS.len() {
        return Err(error("BRIDGE_RELATION_MISMATCH", "bridge_relations"));
    }
    for index in 0..CONTEXT_DIMENSIONS.len() {
        if input.historical_source_values[index] != input.bridge_source_values[index] {
            return Err(error("MEMBER_SOURCE_CONTEXT_MISMATCH", &format!("context[{index}]")));
        }
        if input.theorem_context_values[index] != input.bridge_reviewed_values[index] {
            return Err(error("MEMBER_REVIEWED_CONTEXT_MISMATCH", &format!("context[{index}]")));
        }
        let expected = if input.bridge_source_values[index] == input.bridge_reviewed_values[index]
        {
            ContextBridgeRelationV2::ExactMatch
        } else {
            ContextBridgeRelationV2::ReviewedDivergence
        };
        if input.bridge_relations[index] != expected {
            return Err(error("BRIDGE_RELATION_MISMATCH", &format!("context[{index}]")));
        }
    }
    if input.theorem_temporal_values.len() != TEMPORAL_SEMANTICS.len()
        || input.bridge_temporal_values.len() != TEMPORAL_SEMANTICS.len()
    {
        return Err(error("MEMBER_TEMPORAL_MISMATCH", "temporal_values"));
    }
    for index in 0..TEMPORAL_SEMANTICS.len() {
        if input.theorem_temporal_values[index] != input.bridge_temporal_values[index] {
            return Err(error("MEMBER_TEMPORAL_MISMATCH", &format!("temporal[{index}]")));
        }
    }
    if input.theorem_preconditions.len() != input.member_preconditions.len() {
        return Err(error("PRECONDITION_COVERAGE", "preconditions"));
    }
    for index in 0..input.theorem_preconditions.len() {
        let theorem = &input.theorem_preconditions[index];
        let member = &input.member_preconditions[index];
        if theorem.precondition_id != member.precondition_id || theorem.value != member.value {
            return Err(error("PRECONDITION_MISMATCH", &format!("preconditions[{index}]")));
        }
    }
    Ok(())
}
~~~

Do not add a Rust filesystem resolver, archive parser, evidence resolver, or
rules interpreter.

- [ ] **Step 4: Run both parity suites.**

~~~powershell
cargo test -p mtgml-persistence context_application_v2_semantic_golden_matrix_matches_python_contract --locked
python -m unittest python.tests.test_context_application_v2_validator.ContextApplicationV2SemanticCoreTests -v
~~~

Expected: both languages accept the two positive cases and return identical
error categories for every negative matrix case.

- [ ] **Step 5: Commit Rust parity.**

~~~powershell
git add crates/mtgml-persistence/src/authority.rs crates/mtgml-persistence/src/tests.rs
git commit -m "feat: add Rust ContextApplicationV2 semantic parity"
~~~

---

### Task 4: Extract the narrow V1 context-member seam

**Files:**
- Modify: `scripts/authority_validator.py`
- Test: `python/tests/test_authority_validator.py`

- [ ] **Step 1: Write the seam RED test.**

Add a test that calls the wished-for method
`validate_context_member_source_contract_v1` with a source timing of
`activation_time`, theorem context timing `not_applicable`, empty
preconditions, and a no-op evidence callback. The common seam must pass because
it does not apply the V1-only theorem/source context-vector equality. Then call
the existing V1 `_validate_context_members` path with the same source/theorem
mismatch and assert `MEMBER_SOURCE_CONTEXT_MISMATCH`.

The V1-shaped member must contain exactly these fields:

~~~python
member = {
    "candidate_id": candidate["candidate_id"],
    "candidate_identity": candidate["candidate_identity"],
    "source_instance_id": instance["source_instance_id"],
    "candidate_universe_binding": {
        "path": candidate_binding.path,
        "schema": candidate_binding.schema_or_null,
        "raw_sha256": candidate_binding.raw_sha256.hex(),
    },
    "context_binding": {
        "arity": "binary",
        "directionality": "directed",
        "participant_roles": participant_roles,
        "host_relationship": "cross_host",
    },
    "precondition_attestations": [],
    "member_evidence_refs": [model_evidence.to_wire()],
    "context_member_attestation": {"slot_attestations": context_slots},
}
~~~

Set `instance["source_context"]["timing"]` to `"activation_time"` and use a
theorem with ten `not_applicable` context values. `context_slots` contains the
fourteen existing V1 slots and uses the theorem values.

Run:

~~~powershell
python -m unittest python.tests.test_authority_validator.AuthorityValidatorTests.test_context_common_seam_does_not_apply_v1_context_equality -v
~~~

Expected: `ERROR` because the seam method does not exist.

- [ ] **Step 2: Add the evidence callback without changing V1 defaults.**

Import `Callable` from `collections.abc` and define:

~~~python
EvidenceResolver: TypeAlias = Callable[[Sequence[EvidenceRefV1], str], None]
~~~

Change `_resolve_member_evidence` to:

~~~python
def _resolve_member_evidence(
    self,
    record: Mapping[str, object],
    label: str,
    *,
    evidence_resolver: EvidenceResolver | None = None,
) -> None:
~~~

Parse the member references and each precondition's references with the
existing `_evidence_ref`. Resolve each list with `evidence_resolver` when
provided; otherwise call `self._resolve_evidence_refs`. Preserve the existing
member and precondition diagnostic labels.

- [ ] **Step 3: Add the shared seam and exact validated-record lookup.**

Add these methods to `AuthorityValidator`:

~~~python
def validate_context_member_source_contract_v1(
    self,
    member: Mapping[str, object],
    theorem: Mapping[str, object],
    resolved: ResolvedSourceInstance,
    label: str,
    *,
    evidence_resolver: EvidenceResolver | None = None,
) -> None:
    self._validate_context_member_binding(member, theorem, resolved, label)
    self._resolve_member_evidence(
        member,
        label,
        evidence_resolver=evidence_resolver,
    )
    self._validate_precondition_match(member, theorem, label, resolved)

def require_validated_record(
    self,
    identity: AuthorityIdentityV1,
    kind: RecordKind,
    label: str,
) -> Mapping[str, object]:
    if not self._records:
        _fail("AUTHORITY_NOT_VALIDATED", "authority must be validated before record lookup")
    record = self._records.get(identity.as_text())
    if record is None or record.kind is not kind:
        _fail("THEOREM_REFERENCE_INVALID", f"{label} references an unknown or wrong-kind record")
    return record.record
~~~

The seam must preserve each precondition kind independently:

~~~text
candidate_relation_shape -> exact existing source-shape check
participant_binding      -> exact existing source-participant check
source_context           -> exact historical SourceInstance.source_context check
b2_boundary              -> existing theorem-side bound-B2 resolution/validation
temporal_semantic        -> existing theorem/attestation behavior only
class_projection         -> existing member-proof requirement and fail-closed behavior
~~~

Do not add a SourceInstance-derived temporal check. Do not add
`member_proof_attestation` to any V2 mapping. A V2 class-projection precondition
must follow the existing missing-proof failure path.

- [ ] **Step 4: Route only the V1 context path through the seam.**

In `_validate_context_members`, replace the direct binding, member-evidence,
and precondition calls with:

~~~python
self.validate_context_member_source_contract_v1(
    member,
    theorem,
    resolved,
    member_label,
)
self._validate_context_values_against_source(context_values, resolved, member_label)
~~~

Keep the existing fourteen V1 slot comparisons unchanged. Do not change
`_validate_context_values_against_source`; V2 must never call it.

- [ ] **Step 5: Run V1 tests and commit the seam.**

~~~powershell
python -m unittest python.tests.test_authority_validator -v
git add scripts/authority_validator.py python/tests/test_authority_validator.py
git commit -m "refactor: share V1 context member validation"
~~~

Expected: all existing V1 tests pass, the common seam accepts the isolated
theorem/source context mismatch, and the V1 context path still rejects it.

---

### Task 5: Expose typed Slice-2 member source resolution

**Files:**
- Modify: `scripts/context_application_v2_resolver.py`
- Test: `python/tests/test_context_application_v2_resolver.py`

- [ ] **Step 1: Write the typed-helper RED test.**

Add `test_member_source_helper_uses_all_v2_member_identity_fields`. Construct
the typed synthetic `ContextApplicationMemberV2` already used by the resolver
tests and call:

~~~python
resolved = resolver.resolve_member_source_instance(member)
~~~

Assert that the returned candidate ID, CandidateIdentity digest reference,
SourceInstance ID, candidate-universe digest, and source row bytes match the
member and synthetic universe. Replace each of the candidate ID, identity
digest, SourceInstance ID, and candidate-universe raw digest and assert that the
existing resolver rejects the mutation.

Run:

~~~powershell
python -m unittest python.tests.test_context_application_v2_resolver.ContextApplicationV2ResolverTests.test_member_source_helper_uses_all_v2_member_identity_fields -v
~~~

Expected: `ERROR` because the typed helper does not exist.

- [ ] **Step 2: Implement the thin helper.**

Add this method to `ContextApplicationV2Resolver`:

~~~python
def resolve_member_source_instance(
    self,
    member: ContextApplicationMemberV2,
) -> ResolvedSourceInstance:
    if not isinstance(member, ContextApplicationMemberV2):
        raise _fail("V2 member source resolution requires ContextApplicationMemberV2")
    binding_values = member.candidate_universe_binding
    if not isinstance(binding_values, list) or len(binding_values) != 3:
        raise _fail("V2 member candidate-universe binding is malformed")
    raw_path, raw_schema, raw_digest = binding_values
    if (
        not isinstance(raw_path, str)
        or not isinstance(raw_schema, str)
        or not isinstance(raw_digest, bytes)
    ):
        raise _fail("V2 member candidate-universe binding has invalid fields")
    binding = SourceBindingDigestV1(
        "candidate_universe",
        raw_path,
        raw_schema,
        raw_digest,
    )
    return self._resolver.resolve_candidate_source_instance(
        member.candidate_id,
        member.candidate_identity_digest_reference.to_wire(),
        member.source_instance_id,
        binding,
    )
~~~

Add `ResolvedSourceInstance` to the existing imports from
`authority_source_resolver`; keep `SourceBindingDigestV1` from
`mtgml.authority`. Do not change the resolver's public closure functions.

Do not add candidate indexing or source parsing here; the existing
`AuthoritySourceResolver` remains the sole source resolver.

- [ ] **Step 3: Run the helper test and commit.**

~~~powershell
python -m unittest python.tests.test_context_application_v2_resolver.ContextApplicationV2ResolverTests.test_member_source_helper_uses_all_v2_member_identity_fields -v
git add scripts/context_application_v2_resolver.py python/tests/test_context_application_v2_resolver.py
git commit -m "feat: expose typed V2 member source resolution"
~~~

Expected: the focused test passes and all failures are fail-closed resolver
errors from the existing candidate/source implementation.

---

### Task 6: Add the Python integrated V2 validator

**Files:**
- Modify: `scripts/context_application_v2_validator.py`
- Test: `python/tests/test_context_application_v2_validator.py`

- [ ] **Step 1: Add the read-only synthetic integration fixture factory.**

Keep the factory private to `python/tests/test_context_application_v2_validator.py`.
It must create a temporary repository, copy the existing declared model, write
a two-row REV3 census archive with a valid package manifest, and write a
candidate universe containing these two exact candidate IDs:

~~~text
CROSS_DECK|P1|family.a|family.b|DIRECTIONAL_BINARY
CROSS_DECK|P2|family.a|family.b|DIRECTIONAL_BINARY
~~~

The factory must create one `SourceInstanceV1` for each candidate, set the first
instance's ten source-context values from a function argument, set the second
instance to all `not_applicable`, and return the exact
`AuthoritySourceResolver`, `ContextAuthoritySourceBindingV2` candidate-universe
binding, two typed members, theorem record ID, and V2 record needed by the
tests. Build the V1 base authority with the existing model/roster/acceptance
event pattern, and parameterize the theorem's context timing and V1
`source_context` precondition payload. Use model whole-artifact evidence for
global evidence and candidate-universe JSON pointers to
`/candidates/0/candidate_id` and `/source_instances/0/source_instance_id` for
candidate-local evidence. Construct the V2 record with a valid typed
`ReviewEventRefV3` pointing to a deliberately absent file.

The factory must never write to the repository checkout and must expose a
`file_digests()` snapshot so rejection tests can assert no synthetic source
file changed.

- [ ] **Step 2: Write the primary-entry-point RED tests.**

Add tests for these exact public calls:

~~~python
validator = ContextApplicationV2SemanticValidator(
    source_resolver,
    base_authority_binding=base_binding,
)
result = validator.validate(subject)
self.assertTrue(result.valid)
self.assertEqual(result.member_count, 1)
~~~

The exact-match test must build a `ContextApplicationV2Record` with a
synthetic `ReviewEventRefV3` whose file does not exist. Validation must pass,
proving that Slice 3 uses the event reference only in the recomputed `cpar.v2`
preimage and does not load V3 acceptance data.

The reviewed-divergence test must use source timing `not_applicable`, theorem
timing `activation_time`, bridge source timing `not_applicable`, bridge
reviewed timing `activation_time`, and relation `reviewed_divergence`. Add a
V1 `source_context` precondition with payload `["timing", "not_applicable"]` and
an exact matching member attestation. The validator must pass.

Run:

~~~powershell
python -m unittest python.tests.test_context_application_v2_validator.ContextApplicationV2IntegrationTests.test_exact_match_record_passes -v
~~~

Expected: `ERROR` because `ContextApplicationV2SemanticValidator` does not
exist.

- [ ] **Step 3: Implement exact application and theorem identity handling.**

Add these declarations to `scripts/context_application_v2_validator.py`:

~~~python
@dataclass(frozen=True)
class ContextApplicationV2ValidationResult:
    valid: bool
    member_count: int


class ContextApplicationV2SemanticValidator:
    def __init__(
        self,
        source_resolver: AuthoritySourceResolver,
        *,
        base_authority_binding: ContextAuthoritySourceBindingV2,
    ) -> None:
        self._source_resolver = source_resolver
        self._base_binding = base_authority_binding
        self._v2_resolver = ContextApplicationV2Resolver(
            source_resolver,
            base_authority_binding=base_authority_binding,
        )

    def validate(
        self,
        application: ContextApplicationV2Record | ContextApplicationV2InputV1,
    ) -> ContextApplicationV2ValidationResult:
        if isinstance(application, ContextApplicationV2Record):
            theorem_record_id = application.theorem_record_id
            members = application.members
            application_id = application.application_id
            record_id = application.record_id
        elif isinstance(application, ContextApplicationV2InputV1):
            theorem_record_id = AuthorityIdentityV1(
                AuthorityIdentityKind.CONTEXT_THEOREM_RECORD,
                application.theorem_record_id_bytes,
            )
            members = application.members
            application_id = application.identity()
            record_id = None
        else:
            raise ContextApplicationV2SemanticValidationError(
                "APPLICATION_INPUT_INVALID", "application"
            )

        base_artifact = self._v2_resolver.resolve_source_binding(self._base_binding)
        if not isinstance(base_artifact.json_value, Mapping):
            raise ContextApplicationV2SemanticValidationError(
                "THEOREM_REFERENCE_INVALID", "base_authority_v1"
            )
        base_document = dict(base_artifact.json_value)
        v1_validator = AuthorityValidator(self._source_resolver)
        v1_validator.validate(base_document)
        theorem = v1_validator.require_validated_record(
            theorem_record_id,
            RecordKind.CONTEXT_THEOREM_RECORD,
            "application.theorem_record_id",
        )
        resolved_members: list[ResolvedSourceInstance] = []
        for index, member in enumerate(members):
            try:
                resolved_members.append(
                    self._v2_resolver.resolve_member_source_instance(member)
                )
            except (ContextApplicationV2ResolutionError, ResolutionError) as exc:
                raise ContextApplicationV2SemanticValidationError(
                    getattr(exc, "code", "MEMBER_SOURCE_BINDING_MISMATCH"),
                    f"members[{index}].source_binding",
                ) from exc

        expected_application_id = ContextApplicationV2InputV1(
            theorem_record_id_bytes=theorem_record_id.digest_bytes,
            members=members,
        ).identity()
        if expected_application_id != application_id:
            raise ContextApplicationV2SemanticValidationError(
                "APPLICATION_IDENTITY_MISMATCH", "application_id"
            )
        if isinstance(application, ContextApplicationV2Record):
            expected_record_id = ContextApplicationV2RecordInputV1(
                context_application_id_bytes=application_id.digest_bytes,
                review_event_ref_v3=application.review_event_ref_v3,
            ).identity()
            if expected_record_id != record_id:
                raise ContextApplicationV2SemanticValidationError(
                    "RECORD_IDENTITY_MISMATCH", "record_id"
                )

        for index, (member, resolved) in enumerate(zip(members, resolved_members, strict=True)):
            self._validate_member(member, theorem, resolved, v1_validator, f"members[{index}]")
        return ContextApplicationV2ValidationResult(True, len(members))
~~~

The record branch must use the exact existing
`ContextApplicationV2RecordInputV1`, including `review_event_ref_v3`, and must
not resolve the event. The input branch must use only
`ContextApplicationV2InputV1`; no mapping or duck-typed fallback is allowed.

- [ ] **Step 4: Implement the typed member adapter and pure-core invocation.**

Implement `_validate_member` with these exact operations:

~~~python
def _validate_member(
    self,
    member: ContextApplicationMemberV2,
    theorem: Mapping[str, object],
    resolved: ResolvedSourceInstance,
    v1_validator: AuthorityValidator,
    label: str,
) -> None:
    member_wire = member.to_wire()
    v1_validator.validate_context_member_source_contract_v1(
        member_wire,
        theorem,
        resolved,
        label,
        evidence_resolver=lambda refs, evidence_label: self._resolve_evidence_refs(
            refs, member, evidence_label
        ),
    )
    source_context = resolved.source_instance_record.get("source_context")
    if not isinstance(source_context, Mapping):
        raise ContextApplicationV2SemanticValidationError(
            "MEMBER_SOURCE_CONTEXT_MISMATCH", f"{label}.source_context"
        )
    source_values = tuple(
        self._required_text(source_context.get(name), f"{label}.source_context.{name}")
        for name in CONTEXT_DIMENSIONS
    )
    theorem_context_values = tuple(
        self._required_text(value, f"{label}.theorem.context_dimensions[{index}]")
        for index, value in enumerate(
            self._required_array(theorem.get("context_dimensions"), label)
        )
    )
    theorem_temporal_values = tuple(
        self._required_text(value, f"{label}.theorem.temporal_semantics[{index}]")
        for index, value in enumerate(
            self._required_array(theorem.get("temporal_semantics"), label)
        )
    )
    bridge = member.context_member_bridge_attestation_v2
    semantic_input = ContextApplicationV2SemanticInput(
        theorem_subject_shape=cast(PersistenceValue, theorem["subject_shape"]),
        member_context_binding=cast(PersistenceValue, member_wire["context_binding"]),
        historical_source_values=source_values,
        bridge_source_values=tuple(slot.source_value for slot in bridge.context),
        theorem_context_values=theorem_context_values,
        bridge_reviewed_values=tuple(slot.reviewed_value for slot in bridge.context),
        bridge_relations=tuple(slot.relation for slot in bridge.context),
        theorem_temporal_values=theorem_temporal_values,
        bridge_temporal_values=tuple(slot.reviewed_value for slot in bridge.temporal),
        theorem_preconditions=self._theorem_preconditions(theorem, label),
        member_preconditions=self._member_preconditions(member, label),
    )
    validate_context_application_v2_semantics(semantic_input)
    for index, slot in enumerate(bridge.context):
        self._resolve_evidence_refs(slot.evidence_refs, member, f"{label}.context[{index}]")
    for index, slot in enumerate(bridge.temporal):
        self._resolve_evidence_refs(slot.evidence_refs, member, f"{label}.temporal[{index}]")
~~~

`_theorem_preconditions` extracts each theorem `precondition_id` and `payload`.
`_member_preconditions` extracts fields zero and one of each typed four-field
V1 attestation. Both helpers return tuples in theorem order. Do not call
`_validate_context_values_against_source` from this module.

Define those adapters with exact fail-closed checks:

~~~python
def _required_array(value: object, label: str) -> list[object]:
    if not isinstance(value, list):
        raise ContextApplicationV2SemanticValidationError(
            "PRECONDITION_MISMATCH", label
        )
    return value


def _required_text(value: object, label: str) -> str:
    if not isinstance(value, str) or not value:
        raise ContextApplicationV2SemanticValidationError(
            "THEOREM_REFERENCE_INVALID", label
        )
    return value


def _theorem_preconditions(
    theorem: Mapping[str, object], label: str
) -> tuple[ContextPreconditionValueV1, ...]:
    result: list[ContextPreconditionValueV1] = []
    for index, raw in enumerate(_required_array(theorem.get("preconditions"), label)):
        if not isinstance(raw, Mapping):
            raise ContextApplicationV2SemanticValidationError(
                "PRECONDITION_MISMATCH", f"{label}.preconditions[{index}]"
            )
        result.append(
            ContextPreconditionValueV1(
                precondition_id=_required_text(
                    raw.get("precondition_id"),
                    f"{label}.preconditions[{index}].precondition_id",
                ),
                value=cast(PersistenceValue, raw.get("payload")),
            )
        )
    return tuple(result)


def _member_preconditions(
    member: ContextApplicationMemberV2, label: str
) -> tuple[ContextPreconditionValueV1, ...]:
    result: list[ContextPreconditionValueV1] = []
    for index, raw in enumerate(member.precondition_attestations_v1):
        if not isinstance(raw, list) or len(raw) != 4:
            raise ContextApplicationV2SemanticValidationError(
                "PRECONDITION_MISMATCH", f"{label}.preconditions[{index}]"
            )
        result.append(
            ContextPreconditionValueV1(
                precondition_id=_required_text(
                    raw[0], f"{label}.preconditions[{index}].precondition_id"
                ),
                value=cast(PersistenceValue, raw[1]),
            )
        )
    return tuple(result)
~~~

- [ ] **Step 5: Implement exact evidence resolution and ownership checks.**

Implement `_resolve_evidence_refs` so every reference is resolved with
`ContextApplicationV2Resolver.resolve_evidence`. Wrap resolver failures as
`EVIDENCE_RESOLUTION_FAILURE` while preserving the original exception as the
cause. After resolution, apply only these mechanically encoded candidate-local
checks for the exact candidate-universe JSON path:

~~~python
_CANDIDATE_ID_POINTER = re.compile(r"^/candidates/[0-9]+/candidate_id$")
_CANDIDATE_DIGEST_POINTER = re.compile(
    r"^/candidates/[0-9]+/candidate_identity/digest_hex$"
)
_SOURCE_INSTANCE_ID_POINTER = re.compile(
    r"^/source_instances/[0-9]+/source_instance_id$"
)
_SOURCE_INSTANCE_CANDIDATE_POINTER = re.compile(
    r"^/source_instances/[0-9]+/candidate_id$"
)
~~~

For a `c_candidate` reference to the exact candidate-universe path:

- `_CANDIDATE_ID_POINTER` must resolve to `member.candidate_id`;
- `_CANDIDATE_DIGEST_POINTER` must resolve to
  `member.candidate_identity_digest_reference.digest_bytes.hex()`;
- `_SOURCE_INSTANCE_ID_POINTER` must resolve to `member.source_instance_id`; and
- `_SOURCE_INSTANCE_CANDIDATE_POINTER` must resolve to `member.candidate_id`.

Any mismatch raises `EVIDENCE_SOURCE_SUBSTITUTION`. The numeric path component
selects a JSON record for resolution; it never serves as identity. Whole
artifact references, model references, B1/B2 references, and unknown locator
forms remain global evidence when the accepted locator contract does not encode
candidate ownership. Do not inspect prose or search source text.

Add a two-candidate/two-SourceInstance test that changes the pointer from index
zero to index one and asserts `EVIDENCE_SOURCE_SUBSTITUTION`. Add a global
model whole-artifact evidence test that passes.

- [ ] **Step 6: Run integrated positive tests.**

~~~powershell
python -m unittest python.tests.test_context_application_v2_validator.ContextApplicationV2IntegrationTests -v
~~~

Expected: exact-match and reviewed-divergence cases pass without a V3 event
file. The V1 historical `source_context` precondition case passes only when its
payload equals the historical source value.

---

### Task 7: Complete the semantic negative matrix and V1-kind regressions

**Files:**
- Modify: `python/tests/test_context_application_v2_validator.py`
- Modify: `python/tests/test_authority_validator.py`

- [ ] **Step 1: Add identity and source-binding negatives.**

Use `dataclasses.replace` for structurally valid mutations and construct a new
`ContextApplicationV2Record` when its derived record identity should remain
valid. Use `object.__setattr__` only to simulate a tampered persisted identity
after construction. Assert these outcomes:

| Mutation | Expected code |
| --- | --- |
| unknown or wrong-kind theorem record | `THEOREM_REFERENCE_INVALID` |
| wrong `candidate_id` | existing candidate binding failure wrapped as `MEMBER_SOURCE_BINDING_MISMATCH` |
| wrong full CandidateIdentity digest | existing `CANDIDATE_IDENTITY_MISMATCH` |
| wrong SourceInstance ID | existing `SOURCE_INSTANCE_BINDING_MISMATCH` |
| SourceInstance owned by another candidate | existing `SOURCE_INSTANCE_CANDIDATE_MISMATCH` |
| wrong candidate-universe path/schema/digest | existing candidate-universe binding failure |
| recomputed `cpa.v2` differs from supplied `application_id` | `APPLICATION_IDENTITY_MISMATCH` |
| recomputed `cpar.v2` differs from supplied `record_id` | `RECORD_IDENTITY_MISMATCH` |

The wrong theorem record test must prove that a digest-shaped string alone is
not trusted: exact lookup in the fully validated V1 authority graph must fail.

- [ ] **Step 2: Add bridge and precondition negatives.**

Add one test per row:

| Mutation | Expected code |
| --- | --- |
| member context binding differs from theorem subject | `MEMBER_SUBJECT_MISMATCH` |
| source arity or directionality differs | existing V1 source-shape mismatch |
| participant role, position, kind, or semantic reference differs | existing V1 participant mismatch |
| bridge source value differs from SourceInstance context | `MEMBER_SOURCE_CONTEXT_MISMATCH` |
| bridge reviewed value differs from theorem context | `MEMBER_REVIEWED_CONTEXT_MISMATCH` |
| bridge temporal value differs from theorem temporal value | `MEMBER_TEMPORAL_MISMATCH` |
| `exact_match` with unequal values | `BRIDGE_RELATION_MISMATCH` |
| `reviewed_divergence` with equal values | `BRIDGE_RELATION_MISMATCH` |
| missing or extra precondition attestation | `PRECONDITION_COVERAGE` |
| wrong precondition order or ID | `PRECONDITION_MISMATCH` |
| wrong observed precondition payload | `PRECONDITION_MISMATCH` |
| candidate relation shape source mismatch | existing `PRECONDITION_SOURCE_MISMATCH` |
| participant-binding source mismatch | existing `PRECONDITION_SOURCE_MISMATCH` |
| source-context expected value changed from historical value to reviewed value | existing `PRECONDITION_SOURCE_MISMATCH` |

Add a theorem with a `temporal_semantic` precondition and a matching member
attestation while the SourceInstance has no temporal fact. It must pass the V1
kind-specific portion, proving that no new temporal SourceInstance check was
introduced. Add a theorem with a `class_projection` precondition and a V2
member; it must fail through the existing missing-proof path without a V2 field
or shortcut.

- [ ] **Step 3: Add evidence negatives.**

Mutate one evidence reference at a time and assert:

~~~text
stale member digest                  -> EVIDENCE_RESOLUTION_FAILURE
stale context-slot digest            -> EVIDENCE_RESOLUTION_FAILURE
stale temporal-slot digest           -> EVIDENCE_RESOLUTION_FAILURE
unresolved JSON pointer               -> EVIDENCE_RESOLUTION_FAILURE
wrong source snapshot                 -> EVIDENCE_RESOLUTION_FAILURE
cross-candidate candidate_id pointer  -> EVIDENCE_SOURCE_SUBSTITUTION
cross-SourceInstance ID pointer       -> EVIDENCE_SOURCE_SUBSTITUTION
~~~

Keep one exact global whole-artifact model reference in every positive case so
the tests prove that global evidence is not incorrectly rejected as
candidate-local.

- [ ] **Step 4: Add the unchanged V1 application regression.**

Build a V1 `ContextApplication` member whose theorem context timing is
`activation_time` while the resolved historical source timing is
`not_applicable`. Keep its V1 slot attestation equal to the theorem and its
identity and acceptance fields valid. Assert that `AuthorityValidator` still
rejects it with `MEMBER_SOURCE_CONTEXT_MISMATCH`. This test must use the V1
`context_applications` path, not the new V2 validator.

- [ ] **Step 5: Run the focused negative suite and inspect mutation safety.**

~~~powershell
python -m unittest python.tests.test_context_application_v2_validator -v
python -m unittest python.tests.test_authority_validator -v
~~~

For every rejected typed application, deep-copy the input before validation and
assert that the input and all synthetic repository files are unchanged. The
validator must not write production authority, source, C, event, or review
artifacts.

- [ ] **Step 6: Commit the integrated validator and negative matrix.**

~~~powershell
git add scripts/context_application_v2_validator.py python/tests/test_context_application_v2_validator.py python/tests/test_authority_validator.py
git commit -m "feat: validate ContextApplicationV2 bridge semantics"
~~~

---

### Task 8: Register the fixture and test profile

**Files:**
- Modify: `scripts/run_python_tests.py`
- Modify: `scripts/verify_repository.py`
- Modify: `docs/normative-document-register.v1.json`

- [ ] **Step 1: Add the validator module to the smoke allowlist.**

Insert this exact module name into `SMOKE_TESTS`:

~~~python
"test_context_application_v2_validator",
~~~

Keep the list explicit and do not change full-profile discovery.

- [ ] **Step 2: Require the shared semantic fixture.**

Add this exact path to `verify_repository.py`'s `required` list:

~~~python
"conformance/fixtures/authority/context_application_v2_semantic_golden_matrix.v1.json",
~~~

- [ ] **Step 3: Register this plan.**

Add this exact provisional process entry to
`docs/normative-document-register.v1.json`:

~~~json
{
  "change_process": "process-pr",
  "owner_role": "maintainer",
  "path": "docs/superpowers/plans/2026-09-06-context-application-v2-slice3-semantic-validator.md",
  "role": "process",
  "stability": "provisional"
}
~~~

- [ ] **Step 4: Run registration checks and commit.**

~~~powershell
python scripts/verify_repository.py
python scripts/check_documentation.py
python scripts/validate_schemas.py
git diff --check
git add scripts/run_python_tests.py scripts/verify_repository.py docs/normative-document-register.v1.json
git commit -m "test: register ContextApplicationV2 semantic coverage"
~~~

Expected: repository verification, documentation registration, schema
validation, and diff checks pass. No V2 schema or identity fixture changes are
allowed.

---

### Task 9: Run parity, regression, and required repository gates

**Files:**
- No source edits; inspect the complete diff and generated verification output only.

- [ ] **Step 1: Run focused semantic and Slice-2 regressions.**

~~~powershell
python -m unittest python.tests.test_context_application_v2_validator -v
python -m unittest python.tests.test_authority_validator -v
python -m unittest python.tests.test_authority_source_resolver -v
python -m unittest python.tests.test_context_application_v2_contract -v
python -m unittest python.tests.test_context_application_v2_resolver -v
cargo test -p mtgml-persistence context_application_v2 --all-features --locked
~~~

Expected: all focused tests pass; historical V1, Slice-1, and Slice-2 golden
fixtures remain byte-for-byte unchanged.

- [ ] **Step 2: Run the semantic suite twice and compare results.**

Run the focused Python semantic suite twice from the identical source state.
Save only outputs outside the repository and compare the `valid`, `error_code`,
and matrix-result lines. Do not compare timestamps or process paths. The two
runs must have identical outcomes and error categories.

- [ ] **Step 3: Run Python profiles and language gates.**

~~~powershell
python scripts/run_python_tests.py --profile smoke
python scripts/run_python_tests.py --profile full
ruff format --check python scripts
ruff check python scripts
mypy --config-file python/pyproject.toml
~~~

Record each result separately. A missing tool is `NOT_RUN`, not `PASS`.

- [ ] **Step 4: Run Rust gates.**

~~~powershell
cargo fmt --all -- --check
cargo check --workspace --all-targets --all-features --locked
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --all-features --locked
~~~

If a repository wrapper cannot invoke a required command on Windows, record the
wrapper failure and the separately labeled native command result; do not
promote a substitute to the wrapper gate.

- [ ] **Step 5: Run mechanical and maintainer gates.**

~~~powershell
python scripts/generate_contracts.py --check
python scripts/validate_schemas.py
python scripts/check_documentation.py
python scripts/validate_maintainer_artifacts.py
git diff --check
python scripts/run_checks.py fast
python scripts/run_checks.py integration
python scripts/run_checks.py certification
~~~

The Slice-3 completion rule requires `FAST=PASS`, `INTEGRATION=PASS`, and
`CERTIFICATION=PASS`. If certification cannot execute, retain
`CERTIFICATION=NOT_RUN` or `BLOCKED` and do not claim Slice-3 completion.

- [ ] **Step 6: Run Python compatibility checks.**

Run the full Python profile with each supported interpreter available on the
host: Python 3.11, Python 3.12, and Python 3.13. Report only interpreters
actually executed. Do not infer compatibility from the local default Python.

---

### Task 10: Diff audit and delivery checkpoint

**Files:**
- No source edits unless the audit finds an implementation defect; fix defects with a new RED test first.

- [ ] **Step 1: Audit exact scope.**

~~~powershell
git diff master --stat
git diff master --name-only
git diff master --check
git status --short
~~~

The implementation portion of the diff is limited to the files in the file map
plus the new semantic fixture. The only other branch changes are the approved
design/spec documents and their register entries. The diff must not add
production authority data, V3 reviewer/checklist logic, supersession logic,
HostBinding semantic integration, C changes, Buckle-Up values, cards, rules,
or ML code.

- [ ] **Step 2: Verify preserved artifacts.**

~~~powershell
git diff master -- schemas/context-application-authority.v2.schema.json
git diff master -- docs/adr/0042-context-application-v2-reviewed-context-bridge.md
git diff master -- conformance/fixtures/authority/context_application_v2_identity_golden_matrix.v1.json
git diff master -- conformance/fixtures/authority/context_application_v2_closure_golden_matrix.v1.json
git diff master -- conformance/fixtures/authority/identity_golden_matrix.v1.json
~~~

Expected: no output for all five commands.

- [ ] **Step 3: Keep the delivery checkpoint unmerged.**

~~~powershell
git status --short
git log -5 --oneline
~~~

Keep the branch unmerged. If the user requests a PR after independent review,
push the exact branch and create a PR against `master`; report the exact HEAD
SHA, workflow run ID, and job conclusion after exact-head CI completes. The PR
description must begin with `SLICE 3 ONLY` and list the implemented theorem,
member, historical bridge, reviewed theorem, temporal, relation, evidence, and
Rust/Python parity checks. It must list as not implemented V3 reviewer/checklist
admission, production human acceptance, supersession currentness, HostBinding
semantic integration, production authority records, Buckle-Up review, C
changes, Task 5 Slice 3B, and M3. Do not create production acceptance or human
acceptance state.

## Completion report requirements

Use the exact status fields from the authorized task. Set
`CONTEXT_APPLICATION_V2_SLICE_3_IMPLEMENTED=YES` only when the exact-match,
reviewed-divergence, historical-source-precondition, all required negative,
Rust/Python parity, evidence-identity, and full required gates are `PASS`.

Always retain:

~~~text
CONTEXT_APPLICATION_V2_IMPLEMENTED=NO
FROZEN_CONTRACT=NO
HUMAN_ACCEPTANCE=BLOCKED
CONTEXT_APPLICATION_V2_SLICE_4_AUTHORIZED=NO
TASK_5_SLICE_3B=BLOCKED
M3=BLOCKED
PRODUCTION_AUTHORITY_RECORD_CREATED=NO
ACCEPTANCE_EVENT_CREATED=NO
C_CHANGED=NO
PRODUCTION_BASE_AUTHORITY_PRESENT=NO
PRODUCTION_SEMANTIC_PREFLIGHT=BLOCKED
~~~

If any evidence form used by the implemented fixtures cannot prove
candidate/SourceInstance ownership mechanically, stop with:

~~~text
EVIDENCE_IDENTITY_CONTRACT_GAP=YES
CONTEXT_APPLICATION_V2_SLICE_3_IMPLEMENTED=NO
STATUS=BLOCKED
~~~
