"""Source-bound, rules-neutral host realization resolution for M2.5.C.

All bytes are obtained through ``AuthoritySourceResolver``.  This module only
checks the mechanical mapping/deck/OSI/B2 joins required by a host-binding
claim; it never derives participant roles, causal direction, or C semantics.
"""

from __future__ import annotations

import csv
import io
import json
import sys
from collections.abc import Mapping
from dataclasses import dataclass
from pathlib import Path
from types import MappingProxyType
from typing import cast

ROOT = Path(__file__).resolve().parents[1]
PYTHON_SRC = ROOT / "python" / "src"
if str(PYTHON_SRC) not in sys.path:
    sys.path.insert(0, str(PYTHON_SRC))

from authority_source_resolver import (
    CANDIDATE_IDENTITY_DOMAIN,
    CANDIDATE_IDENTITY_SCHEMA,
    CANDIDATE_UNIVERSE_PATH,
    CANDIDATE_UNIVERSE_SCHEMA,
    AuthoritySourceResolver,
    B2ArtifactBindingsV1,
    ResolutionError,
    ResolvedArtifact,
    ResolvedSourceInstance,
)
from mtgml.authority import SourceBindingDigestV1
from mtgml.host_binding import (
    CrossDeckHostBindingClaimV1,
    CrossDeckParticipantDiscoveryHostBindingV1,
    DiscoveryHostRefV1,
    HostBindingEvidenceRefV2,
    HostBindingSourceBindingV2,
    HostRealizationWitnessV1,
    ParticipantHostRealizationV1,
)


class HostBindingSourceError(ValueError):
    """Raised when verified source records cannot form the requested join."""


@dataclass(frozen=True)
class ResolvedHostRealizationWitness:
    witness: HostRealizationWitnessV1
    discovery_row: Mapping[str, str]
    deck_row: Mapping[str, str]
    osi_record: Mapping[str, object]
    b2_assignments: tuple[Mapping[str, object], ...]


@dataclass(frozen=True)
class ResolvedParticipantHostRealization:
    realization: ParticipantHostRealizationV1
    witnesses: tuple[ResolvedHostRealizationWitness, ...]

    @property
    def host(self) -> DiscoveryHostRefV1:
        return self.realization.host


class HostBindingSourceResolver:
    """Resolve exact host realizations through the accepted source boundary."""

    def __init__(
        self,
        resolver: AuthoritySourceResolver,
        *,
        b2_bindings: B2ArtifactBindingsV1 | None = None,
    ) -> None:
        self._resolver = resolver
        self._b2_bindings = b2_bindings

    def resolve_participant_realization(
        self,
        discovery: CrossDeckParticipantDiscoveryHostBindingV1,
        realization: ParticipantHostRealizationV1,
    ) -> ResolvedParticipantHostRealization:
        if discovery.member_key != realization.member_key:
            self._fail("member key differs between discovery and realization")
        if discovery.participant_position != realization.participant_position:
            self._fail("participant position differs between discovery and realization")
        if discovery.participant_ref != realization.participant_ref:
            self._fail("participant reference differs between discovery and realization")
        if discovery.discovery_host != realization.host:
            self._fail("realization host differs from discovery host")

        mapping_keys = {
            _canonical_ref_key(reference) for reference in discovery.mapping_evidence_refs
        }
        witness_keys = {
            _canonical_ref_key(witness.discovery_mapping_ref) for witness in realization.witnesses
        }
        if len(witness_keys) != len(realization.witnesses):
            self._fail("one discovery mapping cannot back multiple witnesses")
        if mapping_keys != witness_keys:
            self._fail("witness mapping references do not exactly cover discovery mappings")

        resolved: list[ResolvedHostRealizationWitness] = []
        for witness in realization.witnesses:
            resolved.append(self._resolve_witness(discovery, realization, witness))
        return ResolvedParticipantHostRealization(realization, tuple(resolved))

    def resolve_claim(
        self,
        discovery_bindings: tuple[CrossDeckParticipantDiscoveryHostBindingV1, ...],
        realizations: tuple[ParticipantHostRealizationV1, ...],
    ) -> tuple[ResolvedParticipantHostRealization, ...]:
        if len(discovery_bindings) != len(realizations):
            self._fail("claim discovery and realization coverage differs")
        ordered_discovery = sorted(
            discovery_bindings, key=lambda binding: binding.participant_position
        )
        ordered_realizations = sorted(
            realizations, key=lambda realization: realization.participant_position
        )
        if [item.participant_position for item in ordered_discovery] != list(
            range(len(ordered_discovery))
        ):
            self._fail("claim discovery positions are incomplete")
        if [item.participant_position for item in ordered_realizations] != list(
            range(len(ordered_realizations))
        ):
            self._fail("claim realization positions are incomplete")
        return tuple(
            self.resolve_participant_realization(discovery, realization)
            for discovery, realization in zip(ordered_discovery, ordered_realizations, strict=True)
        )

    def resolve_claim_for_member(
        self,
        claim: CrossDeckHostBindingClaimV1,
        candidate_identity_digest: bytes,
        candidate_universe_binding: HostBindingSourceBindingV2,
        pair_aggregates_binding: HostBindingSourceBindingV2,
    ) -> tuple[ResolvedParticipantHostRealization, ...]:
        """Rebind a claim to its exact V1 Candidate/SourceInstance source row."""

        if candidate_universe_binding.artifact_role != "candidate_universe":
            self._fail("candidate source binding does not use candidate_universe role")
        if (
            candidate_universe_binding.path != CANDIDATE_UNIVERSE_PATH
            or candidate_universe_binding.schema_or_null != CANDIDATE_UNIVERSE_SCHEMA
        ):
            self._fail("candidate source binding path or schema is not admitted")
        if pair_aggregates_binding.artifact_role != "rev3_pair_aggregates":
            self._fail("pair aggregate binding does not use the REV3 pair role")
        if candidate_identity_digest != claim.member_key.candidate_identity_digest:
            self._fail("candidate identity digest differs from the claim member key")

        candidate_binding = SourceBindingDigestV1(
            "candidate_universe",
            candidate_universe_binding.path,
            candidate_universe_binding.schema_or_null,
            candidate_universe_binding.raw_sha256,
        )
        candidate_identity = {
            "envelope_id": "mtgml.digest-envelope.v1",
            "algorithm_id": "sha-256",
            "semantic_domain": CANDIDATE_IDENTITY_DOMAIN,
            "payload_codec_id": "mtgml.canonical-cbor.v1",
            "input_schema_id": CANDIDATE_IDENTITY_SCHEMA,
            "digest_hex": candidate_identity_digest.hex(),
        }
        source_instance = self._resolver.resolve_candidate_source_instance(
            claim.member_key.candidate_id,
            candidate_identity,
            claim.member_key.source_instance_id,
            candidate_binding,
        )
        if candidate_identity_digest != bytes.fromhex(
            cast(str, source_instance.candidate.candidate_identity["digest_hex"])
        ):
            self._fail("candidate identity digest differs from the source instance")
        source_row = self._candidate_source_row(source_instance)
        if (
            source_row.get("scope") != "CROSS_DECK"
            or source_row.get("relation") != "DIRECTIONAL_BINARY"
        ):
            self._fail("host binding requires a CROSS_DECK directional candidate source row")
        pair_id = source_row.get("pair_id")
        if not pair_id:
            self._fail("candidate source row has no pair ID")
        pair_artifact = self._resolve_binding(pair_aggregates_binding)
        try:
            pair_value = json.loads(pair_artifact.raw_bytes.decode("utf-8"))
        except (UnicodeDecodeError, json.JSONDecodeError) as exc:
            self._fail(f"REV3 pair aggregate is not valid UTF-8 JSON: {exc}")
        if not isinstance(pair_value, Mapping):
            self._fail("REV3 pair aggregate is not a JSON object")
        pairs = pair_value.get("pairs")
        if not isinstance(pairs, Mapping) or not isinstance(pairs.get(pair_id), Mapping):
            self._fail("candidate pair is absent from REV3 pair aggregates")
        pair = cast(Mapping[str, object], pairs[pair_id]).get("pair")
        if (
            not isinstance(pair, list)
            or len(pair) != 2
            or any(not isinstance(host, str) for host in pair)
        ):
            self._fail("REV3 pair host set is malformed")
        pair_hosts = set(cast(list[str], pair))
        if len(pair_hosts) != 2:
            self._fail("REV3 pair host set is not a cross-deck pair")

        ordered_discovery = sorted(
            claim.discovery_bindings, key=lambda binding: binding.participant_position
        )
        if len(ordered_discovery) != 2:
            self._fail("host binding source row requires exactly two participants")
        for binding in ordered_discovery:
            expected_family = (
                source_row.get("left_family_id")
                if binding.participant_position == 0
                else source_row.get("right_family_id")
            )
            expected_side = (
                "rev3_left_family" if binding.participant_position == 0 else "rev3_right_family"
            )
            if (
                binding.discovery_side != expected_side
                or binding.participant_ref != expected_family
            ):
                self._fail("discovery binding does not match the candidate source row")
            if binding.discovery_host.host_id not in pair_hosts:
                self._fail("discovery host is not one of the candidate pair hosts")

        return self.resolve_claim(
            claim.discovery_bindings,
            claim.participant_host_realizations,
        )

    def _candidate_source_row(self, source_instance: object) -> Mapping[str, str]:
        instance = cast(ResolvedSourceInstance, source_instance)
        raw_ordinal = instance.source_binding.get("row_ordinal")
        if isinstance(raw_ordinal, bool) or not isinstance(raw_ordinal, int):
            self._fail("source instance row ordinal is invalid")
        return self._csv_row(
            instance.source_artifact.raw_bytes,
            raw_ordinal,
            instance.source_artifact.path,
        )

    def _resolve_binding(self, binding: HostBindingSourceBindingV2) -> ResolvedArtifact:
        if binding.artifact_role.startswith("rev3_"):
            return self._resolver.resolve_rev3_member(
                binding.path,
                binding.raw_sha256,
                binding.schema_or_null,
            )
        return self._resolver.resolve_repository_artifact(
            binding.path,
            binding.raw_sha256,
            binding.schema_or_null,
        )

    def resolve_evidence_reference(
        self, reference: HostBindingEvidenceRefV2
    ) -> tuple[ResolvedArtifact, object]:
        """Resolve one typed V2 evidence reference through verified bytes."""

        return self._resolve_ref(reference)

    def _resolve_witness(
        self,
        discovery: CrossDeckParticipantDiscoveryHostBindingV1,
        realization: ParticipantHostRealizationV1,
        witness: HostRealizationWitnessV1,
    ) -> ResolvedHostRealizationWitness:
        discovery_artifact, discovery_value = self._resolve_ref(witness.discovery_mapping_ref)
        deck_artifact, deck_value = self._resolve_ref(witness.deck_row_ref)
        osi_artifact, osi_value = self._resolve_ref(witness.osi_ref)
        del discovery_artifact, deck_artifact, osi_artifact

        discovery_row = self._csv_record(discovery_value, "discovery mapping row")
        deck_row = self._csv_record(deck_value, "deck row")
        osi_record = self._json_record(osi_value, "OSI record")
        self._compare_map_rows(discovery_row, deck_row)

        if discovery_row.get("deck_id") != realization.host.host_id:
            self._fail("deck row host differs from realization host")
        if discovery_row.get("requirement_id") != realization.participant_ref:
            self._fail("deck row requirement differs from participant reference")
        if discovery_row.get("oracle_semantic_identity") != osi_record.get("oracle_id"):
            self._fail("deck row OSI differs from the exact OSI record")

        b2_assignments: list[Mapping[str, object]] = []
        for reference in witness.b2_assignment_refs:
            _, value = self._resolve_ref(reference)
            assignment = self._b2_assignment_record(
                value,
                realization.participant_ref,
                reference,
            )
            if assignment.get("oracle_semantic_identity") != osi_record.get("oracle_id"):
                self._fail("B2 assignment OSI differs from the exact OSI record")
            if assignment.get("requirement_family_id") != realization.participant_ref:
                self._fail("B2 assignment family differs from participant reference")
            b2_assignments.append(MappingProxyType(dict(assignment)))

        return ResolvedHostRealizationWitness(
            witness=witness,
            discovery_row=MappingProxyType(dict(discovery_row)),
            deck_row=MappingProxyType(dict(deck_row)),
            osi_record=MappingProxyType(dict(osi_record)),
            b2_assignments=tuple(b2_assignments),
        )

    def _resolve_ref(self, reference: HostBindingEvidenceRefV2) -> tuple[ResolvedArtifact, object]:
        if reference.artifact_role.startswith("rev3_"):
            artifact = self._resolver.resolve_rev3_member(
                reference.path,
                reference.raw_sha256,
                reference.schema_or_null,
            )
        else:
            artifact = self._resolver.resolve_repository_artifact(
                reference.path,
                reference.raw_sha256,
                reference.schema_or_null,
            )
        kind, payload = reference.locator
        if kind == "whole_artifact":
            value = artifact.json_value if artifact.json_value is not None else artifact.raw_bytes
            return artifact, value
        if kind == "json_pointer":
            result = self._resolver.resolve_locator(
                artifact,
                ("json_pointer", cast(str, payload)),
            )
            return artifact, result.value
        if kind == "csv_row":
            return artifact, self._csv_row(artifact.raw_bytes, cast(int, payload), reference.path)
        if kind in {"jsonl_line", "jsonl_record"}:
            return artifact, self._jsonl_value(artifact.raw_bytes, kind, payload, reference.path)
        self._fail(f"unsupported host-binding locator {kind!r}")

    @staticmethod
    def _csv_row(raw: bytes, index: int, path: str) -> Mapping[str, str]:
        try:
            rows = list(csv.reader(io.StringIO(raw.decode("utf-8"), newline=""), strict=True))
        except (UnicodeDecodeError, csv.Error) as exc:
            raise HostBindingSourceError(f"{path} is not strict UTF-8 CSV: {exc}") from exc
        if not rows or len(rows[0]) != len(set(rows[0])):
            raise HostBindingSourceError(f"{path} has no unique CSV header")
        data = rows[1:]
        if index >= len(data):
            raise HostBindingSourceError(f"{path} CSV row {index} is out of range")
        row = data[index]
        if len(row) != len(rows[0]):
            raise HostBindingSourceError(f"{path} CSV row {index} has the wrong width")
        return dict(zip(rows[0], row, strict=True))

    @staticmethod
    def _jsonl_value(
        raw: bytes,
        kind: str,
        payload: str | int | None,
        path: str,
    ) -> object:
        try:
            lines = raw.decode("utf-8").splitlines()
        except UnicodeDecodeError as exc:
            raise HostBindingSourceError(f"{path} is not UTF-8 JSONL: {exc}") from exc
        if kind == "jsonl_line":
            index = cast(int, payload)
            if index >= len(lines):
                raise HostBindingSourceError(f"{path} JSONL line {index} is out of range")
            selected = lines[index]
        else:
            wanted = cast(str, payload)
            matches = [
                line
                for line in lines
                if isinstance((candidate := _json_object(line, path)).get("id"), str)
                and candidate["id"] == wanted
            ]
            if len(matches) != 1:
                raise HostBindingSourceError(f"{path} JSONL record {wanted!r} is not unique")
            selected = matches[0]
        return _json_object(selected, path)

    @staticmethod
    def _csv_record(value: object, label: str) -> dict[str, str]:
        if not isinstance(value, Mapping) or any(
            not isinstance(key, str) or not isinstance(item, str) for key, item in value.items()
        ):
            raise HostBindingSourceError(f"{label} is not a string-keyed CSV record")
        return {cast(str, key): cast(str, item) for key, item in value.items()}

    @staticmethod
    def _json_record(value: object, label: str) -> dict[str, object]:
        if not isinstance(value, Mapping):
            raise HostBindingSourceError(f"{label} is not a JSON object")
        return {cast(str, key): item for key, item in value.items()}

    def _b2_assignment_record(
        self,
        value: object,
        participant_ref: str,
        reference: HostBindingEvidenceRefV2,
    ) -> dict[str, object]:
        record = self._json_record(value, "B2 assignment")
        if "requirement_assignments" not in record:
            raise HostBindingSourceError(
                "B2 assignment evidence must identify its complete classification record"
            )
        oracle_identity = record.get("oracle_semantic_identity")
        assignments = record.get("requirement_assignments")
        if not isinstance(oracle_identity, str) or not isinstance(assignments, list):
            raise HostBindingSourceError("B2 classification evidence is malformed")
        if self._b2_bindings is not None:
            classification_identity = record.get("classification_identity")
            if not isinstance(classification_identity, Mapping):
                raise HostBindingSourceError("B2 classification identity is missing")
            expected_binding = self._b2_bindings.classifications
            if (
                reference.path != expected_binding.path
                or reference.raw_sha256 != expected_binding.raw_sha256
            ):
                raise HostBindingSourceError(
                    "B2 assignment evidence uses another classification snapshot"
                )
            try:
                classification = self._resolver.resolve_b2_classification(
                    oracle_identity,
                    cast(Mapping[str, object], classification_identity),
                    self._b2_bindings,
                )
                assignment = self._resolver.resolve_b2_assignment(
                    classification,
                    participant_ref,
                    self._b2_bindings,
                )
            except ResolutionError:
                raise
            except (TypeError, ValueError) as exc:
                raise HostBindingSourceError(
                    "B2 assignment is not a valid existing B2 classification edge"
                ) from exc
            result = dict(assignment.assignment)
            result["oracle_semantic_identity"] = oracle_identity
            return result
        matches = [
            self._json_record(item, "B2 requirement assignment")
            for item in assignments
            if isinstance(item, Mapping) and item.get("requirement_family_id") == participant_ref
        ]
        if len(matches) != 1:
            raise HostBindingSourceError(
                "B2 classification has no unique assignment for the participant"
            )
        result = dict(matches[0])
        result["oracle_semantic_identity"] = oracle_identity
        return result

    @staticmethod
    def _compare_map_rows(discovery_row: Mapping[str, str], deck_row: Mapping[str, str]) -> None:
        fields = ("deck_row_id", "deck_id", "oracle_semantic_identity", "requirement_id")
        if any(discovery_row.get(field) != deck_row.get(field) for field in fields):
            raise HostBindingSourceError("discovery mapping and deck row do not form one join")

    @staticmethod
    def _fail(message: str) -> None:
        raise HostBindingSourceError(message)


def _canonical_ref_key(reference: HostBindingEvidenceRefV2) -> bytes:
    from mtgml.persistence import encode_canonical

    return encode_canonical(reference.to_cbor())


def _json_object(line: str, path: str) -> dict[str, object]:
    try:
        value = json.loads(line)
    except json.JSONDecodeError as exc:
        raise HostBindingSourceError(f"{path} contains invalid JSONL: {exc}") from exc
    if not isinstance(value, dict):
        raise HostBindingSourceError(f"{path} JSONL value is not an object")
    return cast(dict[str, object], value)


__all__ = [
    "HostBindingSourceError",
    "HostBindingSourceResolver",
    "ResolvedHostRealizationWitness",
    "ResolvedParticipantHostRealization",
]
