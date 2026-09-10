from __future__ import annotations

import sys
import unittest
from pathlib import Path
from types import SimpleNamespace

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts"))
sys.path.insert(0, str(ROOT / "python" / "tests"))

from context_application_v3_resolver import (
    ContextApplicationV3AuthorityResolver,
    ContextApplicationV3ResolutionError,
)
from mtgml.authority import (
    ApplicationHostBindingV3,
    AuthorityIdentityKind,
    AuthorityIdentityV1,
    ContextApplicationAuthorityV3,
    ContextAuthoritySourceBindingV3,
)


def binding(role: str, digest: bytes) -> ContextAuthoritySourceBindingV3:
    values = {
        "base_authority_v1": (
            "sources/m2_5/authorities/interaction_review_authority.v1.json",
            "manafold.m2.5.c.interaction-review-authority.v1",
        ),
        "candidate_universe": (
            "sources/m2_5/closures/C/interaction_candidate_universe.v2.json",
            "manafold.m2.5.c.interaction-candidate-universe.v2",
        ),
        "relation_authority_v2": (
            "sources/m2_5/authorities/relation_application_authority/v2/relation_application_authority.v2.json",
            "manafold.m2.5.c.relation-application-authority.v2",
        ),
    }
    path, schema = values[role]
    return ContextAuthoritySourceBindingV3(role, path, schema, digest)


class ContextApplicationAuthorityV3Tests(unittest.TestCase):
    def test_empty_authority_has_exact_projection_shape(self) -> None:
        base = binding("base_authority_v1", b"a" * 32)
        candidate = binding("candidate_universe", b"b" * 32)
        relation = binding("relation_authority_v2", b"c" * 32)
        source_bindings = tuple(
            sorted((base, candidate, relation), key=lambda item: item.to_cbor())
        )
        authority = ContextApplicationAuthorityV3(
            base,
            candidate,
            source_bindings,
            relation,
            (),
            None,
            (),
            (),
            (),
            (),
        )
        wire = authority.to_wire()
        self.assertEqual(wire["schema"], "manafold.m2.5.c.context-application-authority.v3")
        self.assertEqual(wire["context_application_v3_records"], [])

    def test_shared_snapshot_mismatch_fails_before_composition(self) -> None:
        base = binding("base_authority_v1", b"a" * 32)
        candidate = binding("candidate_universe", b"b" * 32)
        authority = SimpleNamespace(
            base_authority_v1_binding=base,
            candidate_universe_binding=candidate,
        )
        relation_authority = SimpleNamespace(
            base_authority_v1_binding=base,
            candidate_universe_binding=binding("candidate_universe", b"z" * 32),
        )
        with self.assertRaises(ContextApplicationV3ResolutionError) as raised:
            ContextApplicationV3AuthorityResolver.require_shared_snapshots(
                authority, relation_authority
            )
        self.assertEqual(raised.exception.code, "CONTEXT_APPLICATION_V3_SHARED_SNAPSHOT_MISMATCH")

    def test_v3_host_link_is_typed_to_cpa_semantic_identity(self) -> None:
        identity = AuthorityIdentityV1(AuthorityIdentityKind.CONTEXT_APPLICATION_V3, b"d" * 32)
        link = ApplicationHostBindingV3(
            "context_application_v3",
            identity,
            ("hbc.v1/" + "e" * 64,),
        )
        self.assertEqual(link.to_cbor()[1], b"d" * 32)
        self.assertNotEqual(link.to_cbor()[1], identity.as_text())


if __name__ == "__main__":
    unittest.main()
