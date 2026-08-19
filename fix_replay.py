import re

with open('python/src/mtgml/replay.py', 'r') as f:
    content = f.read()

# Find and replace the validate method in ReplayManifestV2
old = '''    def validate(self) -> None:
        if self.randomness.contract_id != "mtgml.rng.v1":
            raise WireError("semantic.replay_manifest", "unsupported RNG contract in replay")




    def to_wire(self) -> dict[str, object]:
        return {'''

new = '''    def validate(self) -> None:
        if self.randomness.contract_id != "mtgml.rng.v1":
            raise WireError("semantic.replay_manifest", "unsupported RNG contract in replay")
        if not self.decks:
            raise WireError("semantic.replay_manifest", "decks must not be empty")
        seen_players = set()
        for deck in self.decks:
            if deck.player in seen_players:
                raise WireError("semantic.replay_manifest", "duplicate player in decks")
            seen_players.add(deck.player)
            if not deck.deck_id:
                raise WireError("semantic.replay_manifest", "deck_id must not be empty")
        if self.schemas.replay_step != "replay-step.v2":
            raise WireError("semantic.replay_manifest", "replay_step must be replay-step.v2")


    def to_wire(self) -> dict[str, object]:
        self.validate()
        return {'''

if old in content:
    content = content.replace(old, new)
    with open('python/src/mtgml/replay.py', 'w') as f:
        f.write(content)
    print("Fixed!")
else:
    print("Pattern not found!")