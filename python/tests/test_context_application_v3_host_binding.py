from __future__ import annotations

import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts"))
sys.path.insert(0, str(ROOT / "python" / "tests"))

from authority_v2_validator import HostBindingAuthorityV2ReadModel
from context_application_v3_host_binding import validate_application_host_binding_v3
from mtgml.authority import (
    ApplicationHostBindingV3,
    AuthorityIdentityKind,
    AuthorityIdentityV1,
    ContextApplicationV3InputV1,
    ContextApplicationV3Record,
)
from mtgml.host_binding import ApplicationMemberKeyV1, HostBindingSourceBindingV2
from test_context_application_v2_host_binding import _cross_host_claim
from test_context_application_v3_contract import event_ref, member


class ContextApplicationV3HostBindingTests(unittest.TestCase):
    def test_cross_deck_member_requires_exact_v3_link_and_claim_key(self) -> None:
        application = ContextApplicationV3InputV1(b"t" * 32, (member(),))
        record = ContextApplicationV3Record.from_parts(
            application.identity(),
            AuthorityIdentityV1(AuthorityIdentityKind.CONTEXT_THEOREM_RECORD, b"t" * 32),
            application.members,
            event_ref(),
        )
        key = ApplicationMemberKeyV1(
            member().candidate_id,
            member().candidate_identity_digest_reference.digest_bytes,
            member().source_instance_id,
        )
        claim = _cross_host_claim(key)
        claim_id = claim.identity().as_text()
        link = ApplicationHostBindingV3(
            "context_application_v3",
            record.application_id,
            (claim_id,),
        )
        read_model = HostBindingAuthorityV2ReadModel(
            base_authority_v1_binding=HostBindingSourceBindingV2(
                "base_authority_v1",
                "sources/m2_5/authorities/interaction_review_authority.v1.json",
                "manafold.m2.5.c.interaction-review-authority.v1",
                b"b" * 32,
            ),
            candidate_universe_binding=HostBindingSourceBindingV2(
                "candidate_universe",
                "sources/m2_5/closures/C/interaction_candidate_universe.v2.json",
                "manafold.m2.5.c.interaction-candidate-universe.v2",
                b"u" * 32,
            ),
            admitted_claims_by_id=((claim_id, claim),),
            current_claims_by_id=((claim_id, claim),),
            current_claims_by_member=((key, claim_id),),
            claim_record_status_by_record_id=(),
            claim_record_ids_by_claim_id=(),
            used_source_bindings=(),
        )
        result = validate_application_host_binding_v3(
            record,
            link,
            read_model,
            {
                record.members[0].candidate_id: {
                    "scope": "cross_deck",
                    "relation": "directional_binary",
                }
            },
        )
        self.assertEqual(result.application_id, record.application_id.as_text())

    def test_host_binding_is_not_role_authority(self) -> None:
        application = ContextApplicationV3InputV1(b"t" * 32, (member(),))
        record = ContextApplicationV3Record.from_parts(
            application.identity(),
            AuthorityIdentityV1(AuthorityIdentityKind.CONTEXT_THEOREM_RECORD, b"t" * 32),
            application.members,
            event_ref(),
        )
        key = ApplicationMemberKeyV1(
            member().candidate_id,
            member().candidate_identity_digest_reference.digest_bytes,
            member().source_instance_id,
        )
        claim = _cross_host_claim(key)
        claim_id = claim.identity().as_text()
        link = ApplicationHostBindingV3(
            "context_application_v3",
            record.application_id,
            (claim_id,),
        )
        read_model = HostBindingAuthorityV2ReadModel(
            base_authority_v1_binding=HostBindingSourceBindingV2(
                "base_authority_v1",
                "sources/m2_5/authorities/interaction_review_authority.v1.json",
                "manafold.m2.5.c.interaction-review-authority.v1",
                b"b" * 32,
            ),
            candidate_universe_binding=None,
            admitted_claims_by_id=((claim_id, claim),),
            current_claims_by_id=(),
            current_claims_by_member=(),
            claim_record_status_by_record_id=(),
            claim_record_ids_by_claim_id=((claim_id, ("hbcr.v1/" + "f" * 64,)),),
            used_source_bindings=(),
        )
        with self.assertRaisesRegex(Exception, "HOST_CLAIM_NOT_CURRENT"):
            validate_application_host_binding_v3(
                record,
                link,
                read_model,
                {
                    record.members[0].candidate_id: {
                        "scope": "cross_deck",
                        "relation": "directional_binary",
                    }
                },
            )


if __name__ == "__main__":
    unittest.main()
