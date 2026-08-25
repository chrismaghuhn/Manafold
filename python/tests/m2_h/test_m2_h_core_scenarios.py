"""H.4-i core lockstep twin scenarios over the REAL M2 semantic adapter.

These scenarios require the built adapter binary. At COLLECTION time
(module level) the guard resolves ``MTGML_M2_ADAPTER_BIN`` and, when it is
unset or not an existing file, raises ``unittest.SkipTest``, so casual
suite runs skip honestly while a gate runner that sets the variable and
asserts zero skipped statistics per node cannot be fooled.

Every choice below is constructed HERE from public PlayerDecisionRequestV2
data only; the client library carries no choosing logic.
"""

from __future__ import annotations

import contextlib
import io
import os
import unittest
from pathlib import Path

from mtgml._m2_adapter.process import BINARY_ENV_VAR
from mtgml._m2_adapter.protocol import (
    CMD_INFORMATION_STATE,
    CMD_SUBMIT,
    CMD_VISIBLE_DECISION,
    FIELD_INFORMATION_STATE_WIRE_B64,
    FIELD_STEP_WIRE_B64,
    FIELD_VISIBLE_DECISION_WIRE_B64,
)

from m2_h import harness

_BINARY = os.environ.get(BINARY_ENV_VAR)
if _BINARY is None or not Path(_BINARY).is_file():
    raise unittest.SkipTest(
        "M2.H core scenarios require the adapter binary; set MTGML_M2_ADAPTER_BIN"
    )

SEED_HEX = "11" * 32
PLAYER_ONE = "1"
PLAYER_TWO = "2"


class ResetDeterminismTests(unittest.TestCase):
    """S1: identical trusted reset inputs must yield byte-identical twins."""

    def test_twin_payloads_are_byte_equal_and_the_seed_never_leaks(self) -> None:
        with harness.build_twin_clients(SEED_HEX) as twins:
            decisions: dict[str, bytes | None] = {}
            for perspective, left, right in (
                (PLAYER_ONE, twins.client_a1, twins.client_b1),
                (PLAYER_TWO, twins.client_a2, twins.client_b2),
            ):
                context = f"player {perspective}"
                observation_left = left.observation()
                observation_right = right.observation()
                self.assertEqual(
                    harness.wire_payload_bytes(observation_left),
                    harness.wire_payload_bytes(observation_right),
                    f"{context}: observation payloads diverge",
                )
                state_left = left.information_state()
                state_right = right.information_state()
                self.assertEqual(
                    harness.wire_payload_bytes(state_left),
                    harness.wire_payload_bytes(state_right),
                    f"{context}: information-state payloads diverge",
                )
                decision_left = left.visible_decision()
                decision_right = right.visible_decision()
                self.assertEqual(
                    harness.optional_wire_payload_bytes(decision_left),
                    harness.optional_wire_payload_bytes(decision_right),
                    f"{context}: visible-decision payloads diverge",
                )
                decisions[perspective] = harness.optional_wire_payload_bytes(decision_left)
            self.assertIsNotNone(decisions[PLAYER_ONE])
            self.assertIsNone(decisions[PLAYER_TWO])
            results_a = harness.recorded_results(twins.env_a)
            results_b = harness.recorded_results(twins.env_b)
            self.assertGreater(len(results_a), 0)
            for index, (result_a, result_b) in enumerate(zip(results_a, results_b, strict=True)):
                harness.assert_payload_equality(
                    self,
                    result_a,
                    result_b,
                    f"twin result {index} diverges beyond its payload fields",
                )
                self.assertNotIn(
                    harness.SEED_ABSENCE_PROBE_KEY,
                    result_a,
                    "a command result exposed a root_seed key",
                )


class ExplicitDecisionChainTests(unittest.TestCase):
    """S3: one real episode driven through all four decision families."""

    def test_chain_walks_choose_one_number_many_order_then_rests_running(self) -> None:
        with harness.build_twin_clients(SEED_HEX) as twins:
            harness.assert_public_surface_unchanged(self, twins.client_a1, twins.env_a)
            observed_kinds: list[str] = []
            request = twins.client_a1.visible_decision()
            final_step = None
            for expected_kind in harness.EXPECTED_DECISION_KIND_SEQUENCE:
                self.assertIsNotNone(request)
                actual_kind = request.decision.kind
                self.assertEqual(actual_kind, expected_kind)
                observed_kinds.append(actual_kind)
                answer = harness.ANSWER_CONSTRUCTORS[actual_kind](request)
                step = twins.client_a1.submit(harness.response_for(request, answer))
                self.assertEqual(step.submission.kind, "accepted")
                final_step = step
                request = step.next_decision
            self.assertEqual(observed_kinds, list(harness.EXPECTED_DECISION_KIND_SEQUENCE))
            assert final_step is not None
            self.assertIsNone(final_step.next_decision)
            self.assertIsNone(twins.client_a1.visible_decision())
            self.assertEqual(final_step.status.kind, "running")


class AcceptedParityLockstepTests(unittest.TestCase):
    """S4: token route (twin A) versus trusted direct-call route (twin B)."""

    def test_token_route_and_trusted_direct_route_stay_byte_locked(self) -> None:
        with harness.build_twin_clients(SEED_HEX) as twins:
            request_a = twins.client_a1.visible_decision()
            stages = 0
            while True:
                raw_request_b = harness.payload_field(
                    harness.direct_call_result(twins.env_b, CMD_VISIBLE_DECISION, PLAYER_ONE),
                    FIELD_VISIBLE_DECISION_WIRE_B64,
                )
                if request_a is None:
                    self.assertIsNone(
                        raw_request_b,
                        f"terminal stage {stages}: trusted direct-call route returned a "
                        "visible decision while the token route ended",
                    )
                    break
                self.assertIsNotNone(
                    raw_request_b,
                    f"stage {stages}: trusted direct-call route lacks its visible-decision payload",
                )
                kind = request_a.decision.kind
                self.assertEqual(kind, harness.EXPECTED_DECISION_KIND_SEQUENCE[stages])
                self.assertEqual(
                    harness.optional_wire_payload_bytes(request_a),
                    raw_request_b,
                    f"stage {stages} ({kind}): visible-decision payloads diverge before submit",
                )
                request_b = harness.decode_request(raw_request_b)
                response_a = harness.response_for(
                    request_a, harness.ANSWER_CONSTRUCTORS[kind](request_a)
                )
                response_b = harness.response_for(
                    request_b, harness.ANSWER_CONSTRUCTORS[kind](request_b)
                )
                step_a = twins.client_a1.submit(response_a)
                raw_step_b = harness.payload_field(
                    harness.direct_call_result(
                        twins.env_b,
                        CMD_SUBMIT,
                        PLAYER_ONE,
                        harness.submit_response_bytes(response_b),
                    ),
                    FIELD_STEP_WIRE_B64,
                )
                self.assertIsNotNone(
                    raw_step_b,
                    f"stage {stages} ({kind}): trusted direct-call submit result lacks its "
                    "step payload",
                )
                step_b = harness.decode_step(raw_step_b)
                self.assertEqual(step_a.submission.kind, "accepted")
                self.assertEqual(step_b.submission.kind, "accepted")
                self.assertEqual(
                    harness.wire_payload_bytes(step_a),
                    raw_step_b,
                    f"stage {stages} ({kind}): step payloads diverge after submit",
                )
                state_a = twins.client_a1.information_state()
                raw_state_b = harness.payload_field(
                    harness.direct_call_result(twins.env_b, CMD_INFORMATION_STATE, PLAYER_ONE),
                    FIELD_INFORMATION_STATE_WIRE_B64,
                )
                self.assertIsNotNone(
                    raw_state_b,
                    f"stage {stages} ({kind}): trusted direct-call information-state result "
                    "lacks its payload",
                )
                state_b = harness.decode_information_state(raw_state_b)
                self.assertEqual(
                    harness.wire_payload_bytes(state_a),
                    raw_state_b,
                    f"stage {stages} ({kind}): refreshed information states diverge",
                )
                self.assertEqual(state_b.perspective, int(PLAYER_ONE))
                self.assertEqual(step_a.status.to_wire(), step_b.status.to_wire())
                request_a = step_a.next_decision
                stages += 1
            self.assertEqual(stages, len(harness.EXPECTED_DECISION_KIND_SEQUENCE))


class TwinTeardownSafetyTests(unittest.TestCase):
    """H.4-i regression: twin teardown can never mask a scenario error."""

    def test_dead_child_surfaces_original_error_and_still_tears_down_both_twins(self) -> None:
        stderr = io.StringIO()
        with (
            contextlib.redirect_stderr(stderr),
            self.assertRaises(RuntimeError) as caught,
            harness.build_twin_clients(SEED_HEX) as twins,
        ):
            child_a = twins.env_a._core._process
            assert child_a is not None, "reset must have spawned twin A's child"
            child_a.kill()
            child_a.wait()
            raise RuntimeError("scenario failure")

        original = caught.exception
        self.assertEqual(str(original), "scenario failure")
        self.assertIsNone(
            getattr(original, "code", None),
            "the original scenario error was masked by a transport error",
        )
        for label, environment in (("A", twins.env_a), ("B", twins.env_b)):
            core = environment._core
            self.assertTrue(core._closed, f"twin {label} transport was left open")
            child = core._process
            self.assertIsNotNone(child)
            assert child is not None
            self.assertIsNotNone(child.poll(), f"twin {label} child was left running")
        stderr_text = stderr.getvalue()
        self.assertIn("twin A environment shutdown", stderr_text)
        self.assertNotIn("twin B", stderr_text)


if __name__ == "__main__":
    unittest.main()
