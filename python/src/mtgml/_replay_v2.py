from __future__ import annotations

from dataclasses import dataclass

from .canonical import (
    require_digest,
    require_exact_keys,
    require_nonempty,
)


@dataclass(frozen=True, slots=True)
class RandomnessIdentityV2:
    contract_id: str
    root_seed_hex: str

    @classmethod
    def from_wire(cls, value: object) -> RandomnessIdentityV2:
        obj = require_exact_keys(value, {"contract_id", "root_seed_hex"})
        return cls(
            require_nonempty(obj["contract_id"], "contract_id"),
            require_digest(obj["root_seed_hex"]),
        )

    def to_wire(self) -> dict[str, object]:
        return {
            "contract_id": require_nonempty(self.contract_id, "contract_id"),
            "root_seed_hex": require_digest(self.root_seed_hex),
        }
