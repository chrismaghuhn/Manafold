"""H.4-ii lockstep-twin scenarios: typed rejection parity, encoder
divergence, and the malformed raw-byte boundary over the REAL adapter.

Like the H.4-i core scenarios this module requires the built adapter
binary and skips honestly at COLLECTION time when ``MTGML_M2_ADAPTER_BIN``
is unset or missing.

Three scenario groups live here:

- **S5 typed rejection parity** over EXACTLY the eight reachable closed
  submission classes. Every row mirrors the trigger shape documented by
  ``SEMANTIC_CASES`` in
  ``crates/mtgml-conformance/src/isolation/rejection.rs`` (classification
  pipeline: ``crates/mtgml-environment/src/synthetic.rs::submit_player_response``),
  is built purely from public ``PlayerDecisionRequestV2`` data, is
  injected IDENTICALLY into both twins at the same stage (twin A token
  route, twin B trusted ``direct_call`` route), and must leave every
  player-visible view byte-identical while both twins stay in lockstep
  through every remaining accepted stage afterwards.
- **S5-D divergence proof** (BLOCKER-2 evidence): typed-but-semantically
  invalid responses that PASS the shape-only submission encoder, are
  rejected exactly once by Rust with a closed code, yet are rejected by
  the full local mtgml validator — proving the two layers diverge
  intentionally.
- **S6 malformed raw-byte boundary** through the package-private
  ``RestrictedPlayerTransport._submit_wire_bytes`` seam: document-level
  and true byte-level corruption classes of canonical response bytes,
  each answered with ``malformed_response``, zero mutation, no step, and
  a healthy session afterwards.

Every trigger below derives from public request data; nothing here
fabricates responses or reaches below the public surfaces.
"""

from __future__ import annotations

import json
import os
import unittest
from collections.abc import Callable
from pathlib import Path
from typing import Final, NamedTuple

from mtgml._m2_adapter import AdapterError, RestrictedPlayerTransport
from mtgml._m2_adapter.process import BINARY_ENV_VAR
from mtgml._m2_adapter.protocol import MALFORMED_RESPONSE
from mtgml._m2_adapter.submission import encode_decision_response_submission_v2
from mtgml.decision import (
    DECISION_RESPONSE_V2_SCHEMA,
    DecisionAnswerV2,
    DecisionResponseV2,
    PlayerDecisionRequestV2,
)
from mtgml.errors import WireError

from m2_h import harness

_BINARY = os.environ.get(BINARY_ENV_VAR)
if _BINARY is None or not Path(_BINARY).is_file():
    raise unittest.SkipTest(
        "M2.H rejection scenarios require the adapter binary; set MTGML_M2_ADAPTER_BIN"
    )

SEED_HEX = "11" * 32

STAGE_ENTRY: Final = 0
STAGE_COUNT: Final = 1
STAGE_MEMBERS: Final = 2
STAGE_ORDER: Final = 3


def _assert_dense_ids(request: PlayerDecisionRequestV2, expected: list[int]) -> None:
    actual = [candidate.candidate_id for candidate in request.candidates]
    # Explicit raise, not an assert statement: this reality check must
    # survive ``python -O`` or the rows below could produce vacuous evidence.
    if actual != expected:
        raise AssertionError(
            "synthetic program drift: candidate surface changed; "
            f"expected dense ids {expected}, observed {actual}"
        )


def _assert_assembly_bounds(request: PlayerDecisionRequestV2) -> None:
    """Reality check for the choose_many/order triggers: driving stage-2
    count to ``MEMBER_COUNT`` must pin inclusive bounds ``{2, 2}`` with two
    dense candidates. This is what keeps below-minimum cardinality, member
    duplication, and noncanonical ordering reachable through public
    choices; if the engine ever offered a degenerate ``{0, 0}`` surface
    these rows must fail loudly instead of producing vacuous evidence.
    Guards are explicit raises (not ``assert``) so they survive ``-O``."""
    if request.decision.minimum != harness.MEMBER_COUNT:
        raise AssertionError(
            "assembly minimum drifted: "
            f"expected MEMBER_COUNT={harness.MEMBER_COUNT}, "
            f"observed {request.decision.minimum!r}"
        )
    if request.decision.maximum != harness.MEMBER_COUNT:
        raise AssertionError(
            "assembly maximum drifted: "
            f"expected MEMBER_COUNT={harness.MEMBER_COUNT}, "
            f"observed {request.decision.maximum!r}"
        )
    _assert_dense_ids(request, list(range(harness.MEMBER_COUNT)))


def select_one_beyond_maximum_offered(request: PlayerDecisionRequestV2) -> DecisionAnswerV2:
    """``invalid_candidate``: exactly one id past the highest offered."""
    _assert_dense_ids(request, [0])
    beyond = max(candidate.candidate_id for candidate in request.candidates) + 1
    return DecisionAnswerV2("select_one", candidate_id=beyond)


def choose_number_against_choose_one(request: PlayerDecisionRequestV2) -> DecisionAnswerV2:
    """``invalid_answer``: shape-valid answer of a DIFFERENT family than
    the current choose_one request kind."""
    assert request.decision.kind == "choose_one", "family-mismatch row left its entry stage"
    return DecisionAnswerV2("choose_number", value=0)


def choose_number_above_maximum(request: PlayerDecisionRequestV2) -> DecisionAnswerV2:
    """``invalid_number``: exactly +1 beyond the declared maximum bound."""
    maximum = request.decision.maximum
    assert maximum is not None, "choose_number request lacks a maximum bound"
    return DecisionAnswerV2("choose_number", value=maximum + 1)


def select_many_duplicate_first(request: PlayerDecisionRequestV2) -> DecisionAnswerV2:
    """``duplicate_assignment`` on the choose_many surface where the
    inclusive cardinality allows >= 2 members."""
    _assert_assembly_bounds(request)
    first = request.candidates[0].candidate_id
    return DecisionAnswerV2("select_many", candidate_ids=(first, first))


def order_duplicate_first(request: PlayerDecisionRequestV2) -> DecisionAnswerV2:
    """``duplicate_assignment`` on the order surface where the inclusive
    cardinality allows >= 2 members."""
    _assert_assembly_bounds(request)
    first = request.candidates[0].candidate_id
    return DecisionAnswerV2("order", candidate_ids=(first, first))


def select_single_member_below_minimum(request: PlayerDecisionRequestV2) -> DecisionAnswerV2:
    """``invalid_cardinality``: FEWER than the request minimum members on
    choose_many (membership/uniqueness/canonicality all hold, so only the
    below-minimum cardinality arm can fire)."""
    _assert_assembly_bounds(request)
    return DecisionAnswerV2("select_many", candidate_ids=(request.candidates[0].candidate_id,))


def select_many_noncanonical_pair(request: PlayerDecisionRequestV2) -> DecisionAnswerV2:
    """``invalid_order``: both ids offered but submitted in NON-ascending
    order — the precise trigger shape of rejection.rs's
    ``choosemany_noncanonical_ordering`` row."""
    _assert_assembly_bounds(request)
    second_first = (request.candidates[1].candidate_id, request.candidates[0].candidate_id)
    return DecisionAnswerV2("select_many", candidate_ids=second_first)


class RejectionRow(NamedTuple):
    name: str
    stage_index: int
    expected_code: str
    answer_builder: Callable[[PlayerDecisionRequestV2], DecisionAnswerV2]
    player: str


FOREIGN_ACTOR_ROW = RejectionRow(
    "unavailable_foreign_actor",
    STAGE_ENTRY,
    "unavailable_decision",
    harness.select_one_first_offered,
    harness.PLAYER_TWO,
)
WRONG_FAMILY_ROW = RejectionRow(
    "invalid_answer_wrong_family",
    STAGE_ENTRY,
    "invalid_answer",
    choose_number_against_choose_one,
    harness.PLAYER_ONE,
)
UNKNOWN_CANDIDATE_ROW = RejectionRow(
    "invalid_candidate_beyond_offered",
    STAGE_ENTRY,
    "invalid_candidate",
    select_one_beyond_maximum_offered,
    harness.PLAYER_ONE,
)
SELECT_MANY_DUPLICATE_ROW = RejectionRow(
    "duplicate_selectmany_member",
    STAGE_MEMBERS,
    "duplicate_assignment",
    select_many_duplicate_first,
    harness.PLAYER_ONE,
)
ORDER_DUPLICATE_ROW = RejectionRow(
    "duplicate_order_member",
    STAGE_ORDER,
    "duplicate_assignment",
    order_duplicate_first,
    harness.PLAYER_ONE,
)
BELOW_MINIMUM_CARDINALITY_ROW = RejectionRow(
    "invalid_cardinality_below_min",
    STAGE_MEMBERS,
    "invalid_cardinality",
    select_single_member_below_minimum,
    harness.PLAYER_ONE,
)
NUMBER_ABOVE_MAXIMUM_ROW = RejectionRow(
    "invalid_number_above_maximum",
    STAGE_COUNT,
    "invalid_number",
    choose_number_above_maximum,
    harness.PLAYER_ONE,
)
NONCANONICAL_SELECT_MANY_ROW = RejectionRow(
    "invalid_order_noncanonical_selectmany",
    STAGE_MEMBERS,
    "invalid_order",
    select_many_noncanonical_pair,
    harness.PLAYER_ONE,
)

REJECTION_ROWS: Final[tuple[RejectionRow, ...]] = (
    FOREIGN_ACTOR_ROW,
    WRONG_FAMILY_ROW,
    UNKNOWN_CANDIDATE_ROW,
    SELECT_MANY_DUPLICATE_ROW,
    ORDER_DUPLICATE_ROW,
    BELOW_MINIMUM_CARDINALITY_ROW,
    NUMBER_ABOVE_MAXIMUM_ROW,
    NONCANONICAL_SELECT_MANY_ROW,
)

STALE_RESUBMISSION_ROW: Final[tuple[str, str]] = (
    "stale_resubmitted_response",
    "stale_decision",
)

EXECUTED_REJECTION_ROWS: Final[tuple[tuple[str, str], ...]] = (
    *((row.name, row.expected_code) for row in REJECTION_ROWS),
    STALE_RESUBMISSION_ROW,
)

REACHABLE_REJECTION_CODES: Final[frozenset[str]] = frozenset(
    {
        "unavailable_decision",
        "stale_decision",
        "invalid_answer",
        "invalid_candidate",
        "duplicate_assignment",
        "invalid_cardinality",
        "invalid_number",
        "invalid_order",
    }
)


def _all_visible_views(
    twins: harness.TwinClients,
) -> dict[tuple[str, str], tuple[bytes | None, bytes | None]]:
    """Complete player-visible surface across both players and routes."""
    return harness.all_visible_views(twins)


def _drive_single_chain_to_stage(
    test: unittest.TestCase,
    client: harness.AdapterPlayerClient,
    stage_index: int,
) -> PlayerDecisionRequestV2:
    """Advance ONE token-route chain through accepted stages until the live
    request sits at ``stage_index`` (used where no twin pair is needed)."""
    request = client.visible_decision()
    for kind in harness.EXPECTED_DECISION_KIND_SEQUENCE[:stage_index]:
        assert request is not None, f"chain ended before the {kind} stage"
        test.assertEqual(request.decision.kind, kind, "stage driving met an unexpected family")
        driver = harness.ACCEPTED_STAGE_DRIVERS[kind]
        step = client.submit(harness.response_for(request, driver(request)))
        test.assertEqual(step.submission.kind, "accepted")
        request = step.next_decision
    assert request is not None, f"no live request at stage index {stage_index}"
    test.assertEqual(
        request.decision.kind,
        harness.EXPECTED_DECISION_KIND_SEQUENCE[stage_index],
        "stage driving landed on the wrong family",
    )
    return request


class TypedRejectionParityTests(unittest.TestCase):
    """S5: each semantically invalid construction classifies to the same
    closed code on both routes, mutates no player-visible byte, and leaves
    both twins in lockstep for every remaining accepted stage."""

    def _run_rejection_row(
        self,
        row: RejectionRow,
        stage_probe: Callable[[harness.TwinClients], None] | None = None,
    ) -> None:
        with harness.build_twin_clients(SEED_HEX) as twins:
            request_a, request_b = harness.drive_twins_to_stage(twins, self, row.stage_index)
            if stage_probe is not None:
                stage_probe(twins)
            answer = row.answer_builder(request_a)
            before = _all_visible_views(twins)
            step_a, step_b = harness.submit_response_both_routes(
                twins, self, request_a, request_b, answer, row.player
            )
            harness.assert_submission_pair(
                self, step_a, step_b, "rejected", row.expected_code, row.name
            )
            self.assertEqual(
                _all_visible_views(twins),
                before,
                f"row {row.name}: a rejected submit mutated a player-visible view",
            )
            harness.finish_lockstep_after(twins, self, row.stage_index)

    def test_unavailable_foreign_actor_row(self) -> None:
        def assert_p2_sees_nothing(twins: harness.TwinClients) -> None:
            self.assertIsNone(
                twins.client_a2.visible_decision(),
                "P2 saw a visible decision while P1 holds the live request",
            )
            raw_p2_direct = harness.payload_field(
                harness.direct_call_result(
                    twins.env_b, harness.CMD_VISIBLE_DECISION, harness.PLAYER_TWO
                ),
                harness.FIELD_VISIBLE_DECISION_WIRE_B64,
            )
            self.assertIsNone(raw_p2_direct, "trusted route exposed a decision to P2")

        self._run_rejection_row(FOREIGN_ACTOR_ROW, assert_p2_sees_nothing)

    def test_invalid_answer_wrong_family_row(self) -> None:
        self._run_rejection_row(WRONG_FAMILY_ROW)

    def test_invalid_candidate_beyond_offered_row(self) -> None:
        self._run_rejection_row(UNKNOWN_CANDIDATE_ROW)

    def test_duplicate_selectmany_member_row(self) -> None:
        self._run_rejection_row(SELECT_MANY_DUPLICATE_ROW)

    def test_duplicate_order_member_row(self) -> None:
        self._run_rejection_row(ORDER_DUPLICATE_ROW)

    def test_invalid_cardinality_below_minimum_row(self) -> None:
        self._run_rejection_row(BELOW_MINIMUM_CARDINALITY_ROW)

    def test_invalid_number_above_maximum_row(self) -> None:
        self._run_rejection_row(NUMBER_ABOVE_MAXIMUM_ROW)

    def test_invalid_order_noncanonical_selectmany_row(self) -> None:
        self._run_rejection_row(NONCANONICAL_SELECT_MANY_ROW)

    def test_stale_decision_resubmits_consumed_response(self) -> None:
        """``stale_decision`` via the task-prescribed trigger: resubmit an
        already-consumed response after advancing."""
        with harness.build_twin_clients(SEED_HEX) as twins:
            request_a, request_b = harness.drive_twins_to_stage(twins, self, STAGE_ENTRY)
            consumed = harness.select_one_first_offered(request_a)
            accepted_a, accepted_b = harness.submit_response_both_routes(
                twins, self, request_a, request_b, consumed
            )
            harness.assert_submission_pair(
                self, accepted_a, accepted_b, "accepted", None, "consumed submission"
            )
            before = _all_visible_views(twins)
            stale_a, stale_b = harness.submit_response_both_routes(
                twins, self, request_a, request_b, consumed
            )
            harness.assert_submission_pair(
                self, stale_a, stale_b, "rejected", "stale_decision", "resubmitted response"
            )
            self.assertEqual(
                _all_visible_views(twins),
                before,
                "the rejected resubmission mutated a player-visible view",
            )
            harness.finish_lockstep_after(twins, self, STAGE_COUNT)

    def test_rows_pin_exactly_the_eight_reachable_classes(self) -> None:
        executed = {code for _, code in EXECUTED_REJECTION_ROWS}
        self.assertEqual(
            executed,
            REACHABLE_REJECTION_CODES,
            "executed rows must cover EXACTLY the eight reachable classes",
        )
        names = [name for name, _ in EXECUTED_REJECTION_ROWS]
        self.assertEqual(len(names), len(set(names)), "row names must be unique")


class SubmissionEncoderDivergenceTests(unittest.TestCase):
    """S5-D (BLOCKER-2 evidence): the shape-only submission encoder and the
    full local validator intentionally diverge on typed-but-invalid data;
    Rust owns the semantic verdict and rejects exactly once."""

    def _divergence_case(
        self,
        stage_index: int,
        answer: Callable[[PlayerDecisionRequestV2], DecisionAnswerV2],
        expected_code: str,
        context: str,
    ) -> None:
        with harness.build_single_client(SEED_HEX) as environment:
            client = environment.bind_player(harness.PLAYER_ONE)
            request = _drive_single_chain_to_stage(self, client, stage_index)
            invalid = DecisionResponseV2(
                DECISION_RESPONSE_V2_SCHEMA,
                request.player_decision_id,
                request.state_revision,
                answer(request),
            )
            encoded = encode_decision_response_submission_v2(invalid)
            self.assertIsInstance(encoded, bytes, f"{context}: encoder refused the instance")
            self.assertTrue(encoded.startswith(b"{"), f"{context}: encoder output malformed")
            for probe_name, probe in (("validate", invalid.validate), ("to_wire", invalid.to_wire)):
                with self.assertRaises(WireError, msg=f"{context}: local {probe_name} accepted"):
                    probe()
            before = harness.token_view_bytes(client)
            step = client.submit(invalid)
            self.assertEqual(
                (step.submission.kind, step.submission.code),
                ("rejected", expected_code),
                f"{context}: Rust classification diverged from the pinned code",
            )
            self.assertEqual(
                harness.token_view_bytes(client),
                before,
                f"{context}: the single rejected submit mutated a player-visible view",
            )
            still_live = client.visible_decision()
            self.assertIsNotNone(still_live, f"{context}: the rejected submit consumed the turn")
            assert still_live is not None
            self.assertEqual(
                harness.wire_payload_bytes(still_live),
                harness.wire_payload_bytes(request),
                f"{context}: the live request changed across the rejection",
            )

    def test_order_duplicate_passes_encoder_and_rejects_once_as_duplicate_assignment(self) -> None:
        def duplicated_order(request: PlayerDecisionRequestV2) -> DecisionAnswerV2:
            _assert_assembly_bounds(request)
            first = request.candidates[0].candidate_id
            return DecisionAnswerV2("order", candidate_ids=(first, first))

        self._divergence_case(STAGE_ORDER, duplicated_order, "duplicate_assignment", "order-dup")

    def test_selectmany_nonascending_passes_encoder_and_rejects_once_as_invalid_order(self) -> None:
        self._divergence_case(
            STAGE_MEMBERS,
            select_many_noncanonical_pair,
            "invalid_order",
            "select-many-nonascending",
        )


Corruptor = Callable[[bytes], bytes]


def _assert_canonical_object(canonical: bytes) -> None:
    assert canonical.startswith(b"{") and canonical.endswith(b"}"), (
        "canonical submission bytes are no longer a JSON object"
    )


def leading_whitespace(canonical: bytes) -> bytes:
    _assert_canonical_object(canonical)
    return b" " + canonical


def wrong_key_order(canonical: bytes) -> bytes:
    _assert_canonical_object(canonical)
    fields = json.loads(canonical.decode("utf-8"))
    order = ("schema_version", "player_decision_id", "state_revision", "answer")
    pieces = [f'"{key}":{json.dumps(fields[key], separators=(",", ":"))}' for key in order]
    corrupted = ("{" + ",".join(pieces) + "}").encode("utf-8")
    # No-op guard as an explicit raise so it survives ``python -O``.
    if corrupted == canonical:
        raise AssertionError("key-order corruptor became a no-op")
    return corrupted


def unknown_field_added(canonical: bytes) -> bytes:
    _assert_canonical_object(canonical)
    return canonical[:-1] + b',"adapter_unknown_field":1}'


def wrong_schema_version(canonical: bytes) -> bytes:
    _assert_canonical_object(canonical)
    corrupted = canonical.replace(
        DECISION_RESPONSE_V2_SCHEMA.encode("utf-8"), b"decision-response.v9"
    )
    # No-op guard as an explicit raise so it survives ``python -O``.
    if corrupted == canonical:
        raise AssertionError("schema-version corruptor became a no-op")
    return corrupted


def truncated_json(canonical: bytes) -> bytes:
    _assert_canonical_object(canonical)
    # The fixed-width [:-8] suffix strip is safe only because every caller
    # first asserts the canonical document is longer than 8 bytes (the
    # "implausibly small" guard in MalformedRawByteBoundaryTests), so this
    # always yields a non-empty, genuinely truncated document.
    return canonical[:-8]


def candidate_id_u32_overflow(canonical: bytes) -> bytes:
    _assert_canonical_object(canonical)
    corrupted = canonical.replace(b'"candidate_id":0', b'"candidate_id":4294967296')
    # No-op guard as an explicit raise so it survives ``python -O``.
    if b'"candidate_id":4294967296' not in corrupted:
        raise AssertionError("u32 overflow corruptor became a no-op")
    return corrupted


def invalid_utf8_sequence(canonical: bytes) -> bytes:
    _assert_canonical_object(canonical)
    split = len(canonical) // 2
    return canonical[:split] + b"\xff\xfe" + canonical[split:]


def embedded_nul(canonical: bytes) -> bytes:
    _assert_canonical_object(canonical)
    split = len(canonical) // 2
    return canonical[:split] + b"\x00" + canonical[split:]


def truncated_multibyte(canonical: bytes) -> bytes:
    _assert_canonical_object(canonical)
    return canonical + b"\xe2\x82"


def arbitrary_garbage_bytes(_canonical: bytes) -> bytes:
    return b"\xde\xad\xbe\xef" * 16


DOCUMENT_CORRUPTION_CLASSES: Final[tuple[tuple[str, Corruptor], ...]] = (
    ("leading_whitespace", leading_whitespace),
    ("wrong_key_order", wrong_key_order),
    ("unknown_field_added", unknown_field_added),
    ("wrong_schema_version", wrong_schema_version),
    ("truncated_json", truncated_json),
    ("candidate_id_u32_overflow", candidate_id_u32_overflow),
)

RAW_BYTE_CORRUPTION_CLASSES: Final[tuple[tuple[str, Corruptor], ...]] = (
    ("invalid_utf8_sequence", invalid_utf8_sequence),
    ("embedded_nul", embedded_nul),
    ("truncated_multibyte", truncated_multibyte),
    ("arbitrary_garbage_bytes", arbitrary_garbage_bytes),
)


class MalformedRawByteBoundaryTests(unittest.TestCase):
    """S6: every corruption class fails closed at layer A with the closed
    wire code, produces no step anywhere, mutates no player-visible byte,
    and leaves the session healthy for subsequent reads."""

    def _run_corruption_classes(
        self, group: str, classes: tuple[tuple[str, Corruptor], ...]
    ) -> None:
        with harness.build_single_client(SEED_HEX) as environment:
            client = environment.bind_player(harness.PLAYER_ONE)
            seam = RestrictedPlayerTransport(client._transport, client._token)
            for name, corruptor in classes:
                request = client.visible_decision()
                self.assertIsNotNone(request, f"{group}/{name}: live request vanished")
                assert request is not None
                canonical = harness.submit_response_bytes(
                    harness.response_for(request, harness.select_one_first_offered(request))
                )
                self.assertGreater(
                    len(canonical), 8, f"{group}/{name}: canonical document implausibly small"
                )
                corrupted = corruptor(canonical)
                before = harness.token_view_bytes(client)
                with self.assertRaises(AdapterError) as caught:
                    seam._submit_wire_bytes(client._token, corrupted)
                self.assertEqual(
                    caught.exception.code,
                    MALFORMED_RESPONSE,
                    f"{group}/{name}: unexpected boundary code",
                )
                self.assertEqual(
                    harness.token_view_bytes(client),
                    before,
                    f"{group}/{name}: the malformed submit mutated a player-visible view",
                )
                healthy = client.visible_decision()
                self.assertIsNotNone(healthy, f"{group}/{name}: session unhealthy afterwards")
                assert healthy is not None
                self.assertEqual(
                    harness.wire_payload_bytes(healthy),
                    harness.wire_payload_bytes(request),
                    f"{group}/{name}: live request drifted after the failed submit",
                )

    def test_document_level_corruption_classes(self) -> None:
        self._run_corruption_classes("document_level", DOCUMENT_CORRUPTION_CLASSES)

    def test_raw_byte_level_corruption_classes(self) -> None:
        self._run_corruption_classes("raw_byte_level", RAW_BYTE_CORRUPTION_CLASSES)


if __name__ == "__main__":
    unittest.main()
