from __future__ import annotations

import json
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "python" / "src"))

from mtgml._observation_m3 import (
    SYNTHETIC_M3_OBSERVATION_SCHEMA,
    SyntheticM3Observation,
    SyntheticM3Priority,
    SyntheticM3TurnPosition,
)
from mtgml._replay_v4 import (
    REPLAY_FILE_SCHEMA_V4,
    REPLAY_MANIFEST_SCHEMA_V4,
    REPLAY_STEP_SCHEMA_V4,
    AuthoritativeReplayV4,
    ReplayManifestV4,
)
from mtgml.episode import EpisodeStatus
from mtgml.errors import WireError
from mtgml.persistence import (
    CHECKPOINT_DOMAIN_V4,
    CHECKPOINT_INPUT_SCHEMA_V4,
    FULL_STATE_DOMAIN_V4,
    FULL_STATE_INPUT_SCHEMA_V4,
    calculate_checkpoint_digest_v3,
    calculate_checkpoint_digest_v4,
)
from mtgml.wire import decode_canonical, encode_canonical

try:
    import jsonschema
except ImportError:  # pragma: no cover
    jsonschema = None


def _m3(kind: str, step: str | None, priority: str, player: int = 1) -> SyntheticM3Observation:
    if kind in {"beginning", "combat", "ending"}:
        assert step is not None
        pos = SyntheticM3TurnPosition(kind, step)
    else:
        pos = SyntheticM3TurnPosition(kind, None)
    if priority == "none":
        pri = SyntheticM3Priority("none", None)
    else:
        pri = SyntheticM3Priority("held_by", player)
    return SyntheticM3Observation(SYNTHETIC_M3_OBSERVATION_SCHEMA, 1, "1", pos, pri)


class M3ObservationParityTests(unittest.TestCase):
    def test_beginning_untap_none_has_exact_rust_bytes(self) -> None:
        obs = _m3("beginning", "untap", "none")
        self.assertEqual(
            encode_canonical(obs),
            b'{"active_player":"1","priority":{"kind":"none"},"schema_version":"synthetic-m3-observation.v1","turn_number":"1","turn_position":{"kind":"beginning","step":"untap"}}',
        )

    def test_precombat_main_has_no_fake_step(self) -> None:
        obs = _m3("precombat_main", None, "none")
        payload = encode_canonical(obs)
        self.assertEqual(
            payload,
            b'{"active_player":"1","priority":{"kind":"none"},"schema_version":"synthetic-m3-observation.v1","turn_number":"1","turn_position":{"kind":"precombat_main"}}',
        )
        self.assertNotIn(b"step", payload)

    def test_held_by_priority_roundtrips(self) -> None:
        obs = _m3("beginning", "untap", "held_by", player=1)
        payload = encode_canonical(obs)
        self.assertIn(b'"held_by"', payload)
        self.assertEqual(decode_canonical("synthetic-m3-observation.v1", payload), obs)

    def test_closed_turn_vocabulary(self) -> None:
        for kind, step in [
            ("beginning", "untap"),
            ("beginning", "upkeep"),
            ("beginning", "draw"),
            ("combat", "beginning_of_combat"),
            ("combat", "declare_attackers"),
            ("combat", "declare_blockers"),
            ("combat", "combat_damage"),
            ("combat", "end_of_combat"),
            ("ending", "end_step"),
            ("ending", "cleanup"),
        ]:
            with self.subTest(kind=kind, step=step):
                obs = _m3(kind, step, "none")
                self.assertEqual(
                    decode_canonical("synthetic-m3-observation.v1", encode_canonical(obs)), obs
                )
        for kind in ("precombat_main", "postcombat_main"):
            with self.subTest(kind=kind):
                obs = _m3(kind, None, "none")
                self.assertEqual(
                    decode_canonical("synthetic-m3-observation.v1", encode_canonical(obs)), obs
                )

    def test_negative_observation_fixtures_reject(self) -> None:
        base = {
            "schema_version": "synthetic-m3-observation.v1",
            "active_player": "1",
            "turn_number": "1",
            "turn_position": {"kind": "beginning", "step": "untap"},
            "priority": {"kind": "none"},
        }

        def raw(mutator) -> bytes:
            import copy

            value = copy.deepcopy(base)
            mutator(value)
            from mtgml.canonical import canonical_json_bytes

            return canonical_json_bytes(value)

        cases = [
            (
                "wrong schema",
                lambda d: d.update({"schema_version": "synthetic-m3-observation.v999"}),
            ),
            ("unknown field", lambda d: d.update({"unknown_field": True})),
            ("unknown kind", lambda d: d.update({"turn_position": {"kind": "middle"}})),
            (
                "fake step",
                lambda d: d.update({"turn_position": {"kind": "precombat_main", "step": "untap"}}),
            ),
            ("invalid priority", lambda d: d.update({"priority": {"kind": "sometimes"}})),
            ("leaked passes", lambda d: d.update({"consecutive_passes": 0})),
            ("noncanonical player", lambda d: d.update({"active_player": "01"})),
            ("noncanonical turn", lambda d: d.update({"turn_number": "01"})),
        ]
        for label, mutator in cases:
            with self.subTest(label=label):
                with self.assertRaises(WireError):
                    decode_canonical("synthetic-m3-observation.v1", raw(mutator))

    def test_player_safe_contract_hides_privileged_fields(self) -> None:
        for privileged in (
            "consecutive_passes",
            "root_seed",
            "root_seed_hex",
            "checkpoint",
            "full_state",
            "full_state_digest",
            "GameObjectId",
            "continuation",
            "authoritative_event",
        ):
            with self.subTest(field=privileged):
                obs = _m3("beginning", "untap", "none")
                wire = obs.to_wire()
                self.assertNotIn(privileged, wire)
                self.assertNotIn(privileged, wire["turn_position"])
                self.assertNotIn(privileged, wire["priority"])

    @unittest.skipIf(jsonschema is None, "jsonschema is not installed")
    def test_m3_schema_is_closed_and_validates_goldens(self) -> None:
        schema = json.loads(
            (ROOT / "schemas" / "synthetic-m3-observation.v1.schema.json").read_text(
                encoding="utf-8"
            )
        )

        def walk(node: object, path: str) -> None:
            if isinstance(node, dict):
                declares = (
                    node.get("type") == "object" or "properties" in node or "required" in node
                )
                if declares and node.get("additionalProperties") is not False:
                    raise AssertionError(f"open object schema at {path}")
                for key, value in node.items():
                    walk(value, f"{path}/{key}")
            elif isinstance(node, list):
                for index, value in enumerate(node):
                    walk(value, f"{path}/{index}")

        walk(schema, "synthetic-m3-observation.v1.schema.json")
        for name in (
            "synthetic-m3-observation-beginning-untap.json",
            "synthetic-m3-observation-precombat-main.json",
            "synthetic-m3-observation-held-by.json",
        ):
            instance = json.loads((ROOT / "wire" / "golden" / name).read_text(encoding="utf-8"))
            with self.subTest(fixture=name):
                jsonschema.Draft202012Validator(schema).validate(instance)


class CheckpointDigestV4Tests(unittest.TestCase):
    def test_v4_identity_constants_are_exact(self) -> None:
        self.assertEqual(CHECKPOINT_DOMAIN_V4, "mtgml.checkpoint-digest.v4")
        self.assertEqual(CHECKPOINT_INPUT_SCHEMA_V4, "environment-checkpoint-digest-input.v4")
        self.assertEqual(FULL_STATE_DOMAIN_V4, "mtgml.full-state-digest.v4")
        self.assertEqual(FULL_STATE_INPUT_SCHEMA_V4, "full-state-digest-input.v4")

    def test_v4_known_answers_match_rust(self) -> None:
        counters = {
            "decisions_submitted": 0,
            "accepted_transitions": 0,
            "rule_events_emitted": 0,
            "resource_units_consumed": 0,
            "wall_clock_elapsed_millis": 0,
        }
        # Cross-language KAT counterpart: crates/mtgml-persistence/tests/p0_red.rs
        # p0_checkpoint_digest_v4_known_answers asserts the same two hashes
        # through the authoritative Rust V4 implementation.
        self.assertEqual(
            calculate_checkpoint_digest_v4(
                "07" * 32, EpisodeStatus.running(), counters, "in-memory-reference", "4"
            ),
            "c880c64a0ce039c87f4b79d9f80280d203e7c394b56f08bdd808e410b7cb023c",
        )
        self.assertEqual(
            calculate_checkpoint_digest_v4(
                "00" * 32, EpisodeStatus.running(), counters, "in-memory-reference", "4"
            ),
            "8e24ae4933f04d8b93f4afa4d76711ce9109112c21a05f8c90164564cf5b5f2c",
        )

    def test_v3_historical_known_answer_is_unchanged(self) -> None:
        counters = {
            "decisions_submitted": 0,
            "accepted_transitions": 0,
            "rule_events_emitted": 0,
            "resource_units_consumed": 0,
            "wall_clock_elapsed_millis": 0,
        }
        self.assertEqual(
            calculate_checkpoint_digest_v3(
                "07" * 32,
                EpisodeStatus.running(),
                counters,
                "mtgml.canonical-cbor.v1",
                "v3",
            ),
            "b0cf94e1f49fb58feb6ebc07d88b2a7e226be78c1ca92ee7b9772d4f51290f6c",
        )

    def test_v4_rejects_v3_digest_identity(self) -> None:
        # A V3 digest recomputed under V4 domains must differ; the V4
        # manifest validation therefore rejects the historical V3 value.
        counters = {
            "decisions_submitted": 0,
            "accepted_transitions": 0,
            "rule_events_emitted": 0,
            "resource_units_consumed": 0,
            "wall_clock_elapsed_millis": 0,
        }
        v4 = calculate_checkpoint_digest_v4(
            "00" * 32, EpisodeStatus.running(), counters, "in-memory-reference", "4"
        )
        self.assertNotEqual(v4, "3d5a04f81ec127ee86be10518b029f01e49fee567b3bbab6799701f55cc30feb")


class ReplayV4ParityTests(unittest.TestCase):
    def test_manifest_binds_m3_and_rejects_m2(self) -> None:
        payload = (ROOT / "wire" / "golden" / "replay-manifest.v4.json").read_bytes()
        manifest = decode_canonical("replay-manifest.v4", payload)
        assert isinstance(manifest, ReplayManifestV4)
        self.assertEqual(manifest.schemas.observation_payload_codec, "synthetic-m3-observation.v1")
        self.assertEqual(encode_canonical(manifest), payload)
        # M2 codec must fail V4 validation.
        import copy
        import json as _json

        from mtgml.canonical import canonical_json_bytes

        raw = _json.loads(payload.decode("utf-8"))
        mutated = copy.deepcopy(raw)
        mutated["schemas"]["observation_payload_codec"] = "synthetic-m2-observation.v1"
        with self.assertRaises(WireError):
            decode_canonical("replay-manifest.v4", canonical_json_bytes(mutated))

    def test_v4_golden_replay_roundtrips(self) -> None:
        for contract, name in [
            ("replay-manifest.v4", "replay-manifest.v4.json"),
            ("authoritative-replay.v4", "authoritative-replay-empty.v4.json"),
        ]:
            with self.subTest(fixture=name):
                payload = (ROOT / "wire" / "golden" / name).read_bytes()
                decoded = decode_canonical(contract, payload)
                self.assertEqual(encode_canonical(decoded), payload)

    def test_v4_negative_fixtures_reject_with_expected_codes(self) -> None:
        manifest = json.loads(
            (ROOT / "wire" / "negative" / "manifest.json").read_text(encoding="utf-8")
        )
        wanted = {
            c["path"] for c in manifest["fixtures"] if ".v4" in c["contract"] or "-v4-" in c["path"]
        }
        self.assertGreaterEqual(len(wanted), 7)
        for case in manifest["fixtures"]:
            if case["path"] not in wanted:
                continue
            with self.subTest(fixture=case["path"]):
                with self.assertRaises(WireError) as caught:
                    decode_canonical(
                        case["contract"], (ROOT / "wire" / "negative" / case["path"]).read_bytes()
                    )
                self.assertEqual(caught.exception.code, case["expected_error_code"])

    def test_v4_schema_identities_are_exact(self) -> None:
        self.assertEqual(REPLAY_MANIFEST_SCHEMA_V4, "replay-manifest.v4")
        self.assertEqual(REPLAY_FILE_SCHEMA_V4, "authoritative-replay.v4")
        self.assertEqual(REPLAY_STEP_SCHEMA_V4, "replay-step.v4")

    @unittest.skipIf(jsonschema is None, "jsonschema is not installed")
    def test_v4_goldens_match_normative_schemas(self) -> None:
        mapping = {
            "replay-manifest.v4": "replay-manifest.v4.schema.json",
            "authoritative-replay.v4": "authoritative-replay.v4.schema.json",
        }
        for contract, schema_name in mapping.items():
            schema = json.loads((ROOT / "schemas" / schema_name).read_text(encoding="utf-8"))
            golden = json.loads(
                (ROOT / "wire" / "golden" / "manifest.json").read_text(encoding="utf-8")
            )
            for case in golden["fixtures"]:
                if case["contract"] != contract:
                    continue
                instance = json.loads(
                    (ROOT / "wire" / "golden" / case["path"]).read_text(encoding="utf-8")
                )
                with self.subTest(fixture=case["path"]):
                    jsonschema.Draft202012Validator(schema).validate(instance)


class InformationStabilityTests(unittest.TestCase):
    def test_frozen_identities_are_unchanged(self) -> None:
        from mtgml._information_v2 import INFORMATION_STATE_SCHEMA_V2
        from mtgml._observation_v1 import OBSERVATION_SCHEMA
        from mtgml._observation_v1 import observation_digest_from_payload

        self.assertEqual(INFORMATION_STATE_SCHEMA_V2, "information-state-envelope.v2")
        self.assertEqual(OBSERVATION_SCHEMA, "observation-envelope.v1")
        # ObservationDigestV1 domain is stable.
        import hashlib

        self.assertEqual(
            observation_digest_from_payload(b"{}"),
            hashlib.sha256(b"mtgml.observation-digest.v1\x00" + b"{}").hexdigest(),
        )


if __name__ == "__main__":
    unittest.main()
