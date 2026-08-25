"""H.5 isolation, paired-seed, and restart-determinism scenarios over the
REAL adapter (plan §H items 2/7/8/9).

Four scenario groups live here:

- **S2 perspective binding / wrong-perspective probe**: inventory-style
  introspection pins ``AdapterPlayerClient`` to EXACTLY one permanent
  token binding with no rebind/perspective-selection surface anywhere;
  with P1 holding the live entry request, P2 sees nothing on EITHER route,
  injecting the P1-derived response under P2's token yields the closed
  ``unavailable_decision`` step, zero player-visible bytes mutate for
  either perspective, no trusted detail reaches any recorded result, and
  both clients keep working for the whole remaining episode afterwards.
- **S7 multi-endpoint isolation + query purity** on ONE environment with
  both perspectives bound: common identity fields agree while
  retained-knowledge content diverges per perspective — proven through
  DECODED STRUCTURES plus marker searches over canonical bytes, not blind
  inequality — and four interleaved read orders never drift from a fresh
  baseline capture.
- **S8 paired hidden variants (axis-05 subset)**: two independent
  environments from DIFFERENT trusted seeds (``"11"*32`` vs ``"22"*32``)
  behind identical players; every initial public view of BOTH
  perspectives AND the complete accepted entry-transition product must
  stay byte-equal unless a legitimately visible random outcome forces an
  explicitly documented scope restriction.
- **S9 restart determinism**: one environment is driven through the
  scripted explicit chain while capturing its concatenated public payload
  sequence, shut down COMPLETELY (child exits), and relaunched fresh on
  identical reset inputs; replaying the identical scripted choices —
  re-derived from live requests, never copied response objects — must
  reproduce the sequence byte-for-byte. Tokens are excluded by design
  (payload-only comparisons throughout).

Like the sibling scenario modules this file requires the built adapter
binary and skips honestly at COLLECTION time when ``MTGML_M2_ADAPTER_BIN``
is unset or missing. Every choice below is constructed from public
``PlayerDecisionRequestV2`` data only; nothing fabricates responses.
"""

from __future__ import annotations

import os
import unittest
from pathlib import Path
from typing import Final

from mtgml._m2_adapter import AdapterError, AdapterPlayerClient
from mtgml._m2_adapter.process import BINARY_ENV_VAR
from mtgml._m2_adapter.protocol import (
    CMD_VISIBLE_DECISION,
    FIELD_VISIBLE_DECISION_WIRE_B64,
    UNKNOWN_TOKEN,
)
from mtgml.canonical import canonical_json_bytes

from m2_h import harness

_BINARY = os.environ.get(BINARY_ENV_VAR)
if _BINARY is None or not Path(_BINARY).is_file():
    raise unittest.SkipTest(
        "M2.H isolation scenarios require the adapter binary; set MTGML_M2_ADAPTER_BIN"
    )

# Intentional same-literal roles: SEED_HEX_A is the general-purpose seed
# shared by the S2/S7/S9 scenarios below, while SEED_PAIR_PRIMARY holds the
# SAME literal BY DESIGN as the primary leg of the S8 paired-seed pair, whose
# twin differs ONLY through SEED_PAIR_SECONDARY. The repeated "11"*32 is not
# duplication drift; do not "deduplicate" these names.
SEED_HEX_A: Final = "11" * 32
SEED_PAIR_PRIMARY: Final = "11" * 32
SEED_PAIR_SECONDARY: Final = "22" * 32

STAGE_ENTRY: Final = 0

READ_OPERATIONS: Final[tuple[str, ...]] = (
    "observation",
    "information_state",
    "visible_decision",
)


class BindingPermanenceTests(unittest.TestCase):
    """S2 part 1: the API surface admits exactly one permanent binding."""

    def test_client_inventory_pins_one_token_and_no_rebinding_surface(self) -> None:
        with harness.build_single_client(SEED_HEX_A) as environment:
            client_p1 = environment.bind_player(harness.PLAYER_ONE)
            client_p2 = environment.bind_player(harness.PLAYER_TWO)
            for label, client in (
                (harness.PLAYER_ONE, client_p1),
                (harness.PLAYER_TWO, client_p2),
            ):
                context = f"player {label} client"
                harness.assert_public_surface_unchanged(self, client, environment)
                # Inventory-style introspection over EVERY reachable name,
                # public and private: nothing may even mention rebinding,
                # tokens, or perspective selection beyond the single token
                # slot itself — so no perspective-selection method can hide
                # anywhere on the surface.
                self.assertEqual(
                    harness.binding_surface_names(client),
                    set(harness.PLAYER_CLIENT_BINDING_INVENTORY),
                    f"{context}: a rebinding or perspective-selection surface appeared",
                )
                self.assertEqual(
                    set(vars(client)),
                    set(harness.PLAYER_CLIENT_SLOT_INVENTORY),
                    f"{context}: instance state grew beyond one transport and one token",
                )
                self.assertIsInstance(client._token, str, context)
                self.assertTrue(client._token, f"{context}: token slot is empty")
                # The client's own bound transport IS the sole raw-byte
                # seam beneath it: it carries ONLY the submit operation and
                # is bound to THIS token, so any other token fails closed
                # locally, without a single round trip.
                seam = client._transport
                self.assertEqual(
                    {name for name in dir(seam) if "submit" in name.lower()},
                    set(harness.RESTRICTED_SEAM_SUBMIT_SURFACE),
                    f"{context}: the restricted seam grew extra submit surfaces",
                )
                round_trips_before = len(harness.recorded_results(environment))
                with self.assertRaises(AdapterError) as caught:
                    seam._submit_wire_bytes("not" + client._token, b"{}")
                self.assertEqual(
                    caught.exception.code,
                    UNKNOWN_TOKEN,
                    f"{context}: foreign-token use did not fail closed locally",
                )
                self.assertEqual(
                    len(harness.recorded_results(environment)),
                    round_trips_before,
                    f"{context}: the local token check leaked a round trip",
                )
            # One binding per client: distinct bindings never share a token.
            self.assertNotEqual(
                client_p1._token,
                client_p2._token,
                "two bound clients were issued the same token",
            )


class WrongPerspectiveProbeTests(unittest.TestCase):
    """S2 part 2: P1's live request is invisible and unusable under P2."""

    def test_wrong_perspective_submit_is_closed_unavailable_and_zero_mutation(self) -> None:
        with harness.build_twin_clients(SEED_HEX_A) as twins:
            request_p1 = twins.client_a1.visible_decision()
            self.assertIsNotNone(request_p1, "the entry decision is missing for P1")
            assert request_p1 is not None
            # P2 sees nothing of P1's request on either route.
            self.assertIsNone(
                twins.client_a2.visible_decision(),
                "P2 saw a visible decision while P1 holds the live request",
            )
            raw_direct_p2 = harness.payload_field(
                harness.direct_call_result(twins.env_b, CMD_VISIBLE_DECISION, harness.PLAYER_TWO),
                FIELD_VISIBLE_DECISION_WIRE_B64,
            )
            self.assertIsNone(raw_direct_p2, "the trusted route exposed a decision to P2")

            before = harness.all_visible_views(twins)
            recorded_before_a = len(harness.recorded_results(twins.env_a))
            recorded_before_b = len(harness.recorded_results(twins.env_b))

            # ONE P1-derived response injected under P2's identity on BOTH
            # routes (token route here, trusted direct-call route below).
            answer = harness.select_one_first_offered(request_p1)
            response_p2 = harness.response_for(request_p1, answer)
            step = twins.client_a2.submit(response_p2)
            self.assertEqual(
                (step.submission.kind, step.submission.code),
                ("rejected", "unavailable_decision"),
                "the wrong-perspective submission was not closed as unavailable_decision",
            )
            raw_step_direct = harness.payload_field(
                harness.direct_call_result(
                    twins.env_b,
                    harness.CMD_SUBMIT,
                    harness.PLAYER_TWO,
                    harness.submit_response_bytes(response_p2),
                ),
                harness.FIELD_STEP_WIRE_B64,
            )
            self.assertIsNotNone(
                raw_step_direct,
                "trusted direct-call wrong-perspective submit lacks its step payload",
            )
            assert raw_step_direct is not None
            self.assertEqual(
                raw_step_direct,
                harness.wire_payload_bytes(step),
                "wrong-perspective step payloads diverge across routes",
            )

            # No trusted detail anywhere in the closed step: neither the
            # trusted seeds/keys/tokens nor ANY fragment of P1's request
            # identity may appear in P2-visible bytes.
            step_blob = harness.wire_payload_bytes(step)
            markers = self._trusted_detail_markers(twins, request_p1)
            for marker in markers:
                self.assertNotIn(marker, step_blob, "trusted detail leaked into P2's step")
            for recorded_environment, recorded_before in (
                (twins.env_a, recorded_before_a),
                (twins.env_b, recorded_before_b),
            ):
                probed = harness.recorded_results(recorded_environment)[recorded_before:]
                self.assertTrue(probed, "the probe produced no recorded results to audit")
                for index, result in enumerate(probed):
                    self.assertNotIn(
                        harness.SEED_ABSENCE_PROBE_KEY,
                        result,
                        "a probe-window result exposed a root_seed key",
                    )
                    serialized = canonical_json_bytes(dict(result))
                    for marker in markers:
                        self.assertNotIn(
                            marker,
                            serialized,
                            f"probe result {index} leaked trusted detail",
                        )

            # Zero player-visible mutation for BOTH perspectives on BOTH routes.
            self.assertEqual(
                harness.all_visible_views(twins),
                before,
                "the wrong-perspective submit mutated a player-visible view",
            )

            # Coexistence: both clients keep working after the probe.
            still_live = twins.client_a1.visible_decision()
            self.assertIsNotNone(still_live, "P1 lost its live request during the probe")
            assert still_live is not None
            self.assertEqual(
                harness.wire_payload_bytes(still_live),
                harness.wire_payload_bytes(request_p1),
                "P1's live request drifted across the probe",
            )
            state_p2 = twins.client_a2.information_state()
            self.assertEqual(
                state_p2.perspective,
                int(harness.PLAYER_TWO),
                "the probed P2 client drifted off its perspective binding",
            )
            harness.finish_lockstep_after(twins, self, STAGE_ENTRY)

    @staticmethod
    def _trusted_detail_markers(
        twins: harness.TwinClients, request_p1: harness.PlayerDecisionRequestV2
    ) -> tuple[bytes, ...]:
        transport_a = twins.env_a._transport
        transport_b = twins.env_b._transport
        return (
            SEED_HEX_A.encode("utf-8"),
            transport_a._trusted_key.encode("utf-8"),
            transport_b._trusted_key.encode("utf-8"),
            twins.client_a1._token.encode("utf-8"),
            twins.client_a2._token.encode("utf-8"),
            twins.client_b1._token.encode("utf-8"),
            twins.client_b2._token.encode("utf-8"),
            f'"player_decision_id":"{request_p1.player_decision_id}"'.encode(),
        )


class MultiEndpointIsolationTests(unittest.TestCase):
    """S7: one environment, both perspectives — agreement, divergence,
    and query purity."""

    def test_public_agreement_and_private_divergence_use_decoded_structures(self) -> None:
        with harness.build_single_client(SEED_HEX_A) as environment:
            client_p1 = environment.bind_player(harness.PLAYER_ONE)
            client_p2 = environment.bind_player(harness.PLAYER_TWO)

            state_p1 = client_p1.information_state()
            state_p2 = client_p2.information_state()

            # Common identity frame agrees across perspectives ...
            self.assertEqual(
                state_p1.schema_version,
                state_p2.schema_version,
                "the two perspectives disagree on the information-state schema",
            )
            self.assertEqual(
                state_p1.state_revision,
                state_p2.state_revision,
                "the two perspectives must observe one shared state revision",
            )
            observation_p1 = client_p1.observation()
            observation_p2 = client_p2.observation()
            self.assertEqual(
                observation_p1.schema_version,
                observation_p2.schema_version,
                "the two perspectives disagree on the observation schema",
            )
            self.assertEqual(
                observation_p1.payload_codec,
                observation_p2.payload_codec,
                "the two perspectives disagree on the observation payload codec",
            )
            self.assertEqual(
                observation_p1.state_revision,
                observation_p2.state_revision,
                "observations disagree on the shared revision",
            )
            # ... while the perspective field differs exactly by design.
            self.assertEqual(state_p1.perspective, int(harness.PLAYER_ONE))
            self.assertEqual(state_p2.perspective, int(harness.PLAYER_TWO))

            # Decision ownership stays asymmetric: P1 owns the live request.
            self.assertIsNotNone(client_p1.visible_decision())
            self.assertIsNone(client_p2.visible_decision())

            # Private divergence proven through DECODED STRUCTURES, never
            # blind byte inequality (the differing perspective labels alone
            # would trivially unequalize the blobs).
            knowledge_p1 = state_p1.retained_knowledge
            knowledge_p2 = state_p2.retained_knowledge
            self.assertEqual(
                len(knowledge_p1),
                1,
                "P1 knowledge-record count drifted from the synthetic program",
            )
            self.assertEqual(
                len(knowledge_p2),
                2,
                "P2 knowledge-record count drifted from the synthetic program",
            )
            shared_public = knowledge_p1[0]
            self.assertEqual(
                (
                    shared_public.kind,
                    shared_public.opaque_object_id,
                    shared_public.known_definition,
                ),
                ("active", 1, 1),
                "the shared battlefield record drifted",
            )
            self.assertEqual(
                knowledge_p2[0].to_wire(),
                shared_public.to_wire(),
                "the shared battlefield record disagrees across perspectives",
            )
            own_library = knowledge_p2[1]
            location = own_library.current_known_location_fact
            self.assertIsNotNone(location, "P2's own-library record lost its location fact")
            assert location is not None
            self.assertEqual(
                (location.location.zone, location.location.player),
                ("library", int(harness.PLAYER_TWO)),
                "P2's second record is not its own library accounting",
            )
            self.assertFalse(
                any(
                    record.current_known_location_fact is not None
                    and record.current_known_location_fact.location.zone == "library"
                    for record in knowledge_p1
                ),
                "P1 must hold no library-zone accounting of its own",
            )

            # Marker search over canonical bytes mirrors the structural split.
            blob_p1 = harness.wire_payload_bytes(state_p1)
            blob_p2 = harness.wire_payload_bytes(state_p2)
            opaque_marker = b'"opaque_object_id":"'
            self.assertEqual(
                blob_p1.count(opaque_marker),
                1,
                "P1's information state must carry exactly one opaque object marker",
            )
            self.assertEqual(
                blob_p2.count(opaque_marker),
                2,
                "P2's information state must carry exactly two opaque object markers",
            )
            self.assertNotIn(
                b'"opaque_object_id":"2"',
                blob_p1,
                "P1's bytes exposed the private second object id",
            )
            self.assertIn(
                b'"opaque_object_id":"2"',
                blob_p2,
                "P2's own-library accounting lost the second object id",
            )
            self.assertNotIn(
                b'"zone":"library"',
                blob_p1,
                "P1's bytes exposed a library-zone record",
            )
            self.assertIn(
                b'"zone":"library"',
                blob_p2,
                "P2's bytes lost their library-zone accounting",
            )
            # With the structural divergence established above, the payload
            # bytes being unequal now carries real evidence instead of just
            # reflecting the by-design perspective label.
            knowledge_slice_p1 = canonical_json_bytes([record.to_wire() for record in knowledge_p1])
            knowledge_slice_p2 = canonical_json_bytes([record.to_wire() for record in knowledge_p2])
            self.assertNotEqual(
                knowledge_slice_p1,
                knowledge_slice_p2,
                "knowledge slices collapsed together across perspectives",
            )
            self.assertNotEqual(
                blob_p1,
                blob_p2,
                "information states collapsed across perspectives",
            )

    def test_interleaved_read_orders_never_drift(self) -> None:
        with harness.build_single_client(SEED_HEX_A) as environment:
            clients = {
                harness.PLAYER_ONE: environment.bind_player(harness.PLAYER_ONE),
                harness.PLAYER_TWO: environment.bind_player(harness.PLAYER_TWO),
            }

            def full_capture() -> dict[tuple[str, str], bytes | None]:
                return {
                    (player, operation): _read_view(clients[player], operation)
                    for player in clients
                    for operation in READ_OPERATIONS
                }

            baseline = full_capture()
            self.assertIsNotNone(
                baseline[(harness.PLAYER_ONE, "visible_decision")],
                "the fresh baseline lost P1's entry decision",
            )
            self.assertIsNone(
                baseline[(harness.PLAYER_TWO, "visible_decision")],
                "the fresh baseline captured a decision for P2",
            )

            for index in range(4):
                offset = index % len(READ_OPERATIONS)
                rotated = READ_OPERATIONS[offset:]
                rotated += READ_OPERATIONS[:offset]
                ordered_players = (
                    (harness.PLAYER_ONE, harness.PLAYER_TWO)
                    if index % 2 == 0
                    else (harness.PLAYER_TWO, harness.PLAYER_ONE)
                )
                order = [(player, op) for op in rotated for player in ordered_players]
                for position, (player, operation) in enumerate(order):
                    view = _read_view(clients[player], operation)
                    self.assertEqual(
                        view,
                        baseline[(player, operation)],
                        f"order {index} step {position} ({operation}, P{player}): "
                        "an interleaved read drifted from the fresh baseline",
                    )
            # Reads never mutated anything: the final capture is identical.
            self.assertEqual(
                full_capture(),
                baseline,
                "interleaved reads mutated a player-visible view",
            )


class PairedHiddenVariantTests(unittest.TestCase):
    """S8 (axis-05 subset): different trusted seeds, identical players."""

    def test_seed_pair_views_and_entry_product_stay_byte_equal(self) -> None:
        with harness.build_twin_clients(SEED_PAIR_PRIMARY, SEED_PAIR_SECONDARY) as twins:
            self._assert_views_byte_equal(twins, "initial")

            request_left = twins.client_a1.visible_decision()
            request_right = twins.client_b1.visible_decision()
            self.assertIsNotNone(request_left, "primary seed lost its entry decision")
            self.assertIsNotNone(request_right, "secondary seed lost its entry decision")
            assert request_left is not None and request_right is not None

            # ONE identical accepted entry submission driven through BOTH
            # environments, each answer derived from its OWN live request.
            step_left = twins.client_a1.submit(
                harness.response_for(request_left, harness.select_one_first_offered(request_left))
            )
            step_right = twins.client_b1.submit(
                harness.response_for(request_right, harness.select_one_first_offered(request_right))
            )
            for label, step in (("primary", step_left), ("secondary", step_right)):
                self.assertEqual(
                    (step.submission.kind, step.submission.code),
                    ("accepted", None),
                    f"{label} seed: the entry submission was not accepted",
                )

            # Empirical axis-05 scope decision. Expected reality (M2.G
            # axis_05 semantics): the entry-stage kernel samples hidden raw
            # words whose occurrence-only projector emits NO envelope, so
            # no visible random outcome exists and every rule-relevant
            # product derives from seed-independent allocator heads. Full
            # byte equality is therefore asserted; if a visible
            # random_outcome_visible ever appears, this scenario MUST be
            # consciously restricted with documented justification instead
            # of silently weakening (fail closed below).
            kinds_left = [event.event.kind for event in step_left.observed_events]
            kinds_right = [event.event.kind for event in step_right.observed_events]
            random_left = sorted(kind for kind in kinds_left if kind == "random_outcome_visible")
            random_right = sorted(kind for kind in kinds_right if kind == "random_outcome_visible")
            self.assertEqual(
                random_left,
                random_right,
                "visible random outcomes disagreed across seeds",
            )
            if random_left:
                self.fail(
                    "axis-05 comparison-scope revision REQUIRED: visible "
                    f"random_outcome_visible events appeared ({random_left}); restrict "
                    "this scenario's byte-equality scope with an explicit documented "
                    "justification before trusting its evidence"
                )
            self.assertEqual(
                kinds_left,
                kinds_right,
                "entry-step event batches diverge across seeds without visible randomness",
            )
            self.assertEqual(
                harness.wire_payload_bytes(step_left),
                harness.wire_payload_bytes(step_right),
                "entry-step payloads diverge across seeds",
            )
            self._assert_views_byte_equal(twins, "refreshed")

    def _assert_views_byte_equal(self, twins: harness.TwinClients, context: str) -> None:
        """The complete view triple of BOTH perspectives must stay byte-equal
        across the paired-seed twins (see the axis-05 scope note above)."""
        for perspective, left, right in (
            (harness.PLAYER_ONE, twins.client_a1, twins.client_b1),
            (harness.PLAYER_TWO, twins.client_a2, twins.client_b2),
        ):
            label = f"{context} seed-pair views (player {perspective})"
            self.assertEqual(
                harness.wire_payload_bytes(left.observation()),
                harness.wire_payload_bytes(right.observation()),
                f"{label}: observations diverge across seeds",
            )
            self.assertEqual(
                harness.wire_payload_bytes(left.information_state()),
                harness.wire_payload_bytes(right.information_state()),
                f"{label}: information states diverge across seeds",
            )
            self.assertEqual(
                harness.optional_wire_payload_bytes(left.visible_decision()),
                harness.optional_wire_payload_bytes(right.visible_decision()),
                f"{label}: decisions diverge across seeds",
            )


class RestartDeterminismTests(unittest.TestCase):
    """S9: shutdown + relaunch reproduces the public payload sequence."""

    def _scripted_episode_sequence(self) -> tuple[bytes | None, ...]:
        """Drive one full scripted episode on a PRIVATE process and return
        its concatenated public payload sequence: the initial views of both
        perspectives followed by, per stage, the request payload, the step
        payload, and a freshly-read post-transition information-state
        payload. The submitted-response echo is deliberately NOT part of
        the sequence, and tokens are excluded by design (payload-only).
        Answers come ONLY from the live request data through the pinned
        stage drivers — no response object is ever carried across runs."""
        environment = harness.RecordingEnvironment()
        pieces: list[bytes | None] = []
        try:
            environment.reset_synthetic(root_seed_hex=SEED_HEX_A)
            client_p1 = environment.bind_player(harness.PLAYER_ONE)
            client_p2 = environment.bind_player(harness.PLAYER_TWO)
            for client in (client_p1, client_p2):
                pieces.append(harness.wire_payload_bytes(client.observation()))
                pieces.append(harness.wire_payload_bytes(client.information_state()))
                pieces.append(harness.optional_wire_payload_bytes(client.visible_decision()))
            request = client_p1.visible_decision()
            for kind in harness.EXPECTED_DECISION_KIND_SEQUENCE:
                self.assertIsNotNone(request, f"chain ended before the {kind} stage")
                assert request is not None
                self.assertEqual(request.decision.kind, kind)
                pieces.append(harness.wire_payload_bytes(request))
                step = client_p1.submit(
                    harness.response_for(request, harness.ACCEPTED_STAGE_DRIVERS[kind](request))
                )
                self.assertEqual(
                    step.submission.kind,
                    "accepted",
                    f"the {kind} stage submission was not accepted",
                )
                pieces.append(harness.wire_payload_bytes(step))
                pieces.append(harness.wire_payload_bytes(client_p1.information_state()))
                request = step.next_decision
            self.assertIsNone(request, "chain continued past the scripted families")
            self.assertIsNone(client_p1.visible_decision())
        finally:
            environment.shutdown()
            environment._core.close()
        core = environment._core
        child = core._process
        self.assertIsNotNone(child, "the episode process never spawned")
        assert child is not None
        self.assertIsNotNone(child.poll(), "the child survived complete shutdown")
        self.assertTrue(core._closed, "the core stayed open after close")
        return tuple(pieces)

    def test_relaunch_reproduces_concatenated_public_sequence(self) -> None:
        sequence_original = self._scripted_episode_sequence()
        # Fresh process, IDENTICAL reset inputs, identical scripted choices
        # re-derived from the relaunched environment's own live requests.
        sequence_replayed = self._scripted_episode_sequence()
        joined_original = b"".join(piece or b"" for piece in sequence_original)
        self.assertGreater(len(joined_original), 0, "the captured public sequence is empty")
        # The per-piece tuple equality subsumes any concatenated-bytes check;
        # keeping only it avoids a logically dead duplicate assertion.
        self.assertEqual(
            sequence_original,
            sequence_replayed,
            "a per-piece public payload diverged between the original run and the restart",
        )


def _read_view(client: AdapterPlayerClient, operation: str) -> bytes | None:
    if operation == "observation":
        return harness.wire_payload_bytes(client.observation())
    if operation == "information_state":
        return harness.wire_payload_bytes(client.information_state())
    if operation == "visible_decision":
        return harness.optional_wire_payload_bytes(client.visible_decision())
    raise AssertionError(f"unknown read operation {operation!r}")


if __name__ == "__main__":
    unittest.main()
