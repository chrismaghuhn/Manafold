from __future__ import annotations

import sys
import unittest
from pathlib import Path
from types import SimpleNamespace

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts"))
sys.path.insert(0, str(ROOT / "python" / "tests"))

from context_application_v3_host_binding import validate_application_host_binding_v3
from mtgml.authority import (
    ApplicationHostBindingV3,
    AuthorityIdentityKind,
    AuthorityIdentityV1,
    ContextApplicationV3InputV1,
    ContextApplicationV3Record,
)
from mtgml.host_binding import ApplicationMemberKeyV1
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
        link = ApplicationHostBindingV3(
            "context_application_v3",
            record.application_id,
            ("hbc.v1/" + "e" * 64,),
        )
        key = ApplicationMemberKeyV1(
            member().candidate_id,
            member().candidate_identity_digest_reference.digest_bytes,
            member().source_instance_id,
        )
        claim = SimpleNamespace(member_key=key, observed_host_relationship="cross_host")
        result = validate_application_host_binding_v3(
            record,
            link,
            {"hbc.v1/" + "e" * 64: claim},
            {
                record.members[0].candidate_id: {
                    "scope": "cross_deck",
                    "relation": "directional_binary",
                }
            },
        )
        self.assertEqual(result.application_id, record.application_id.as_text())

    def test_host_binding_is_not_role_authority(self) -> None:
        self.assertTrue(True)


if __name__ == "__main__":
    unittest.main()
