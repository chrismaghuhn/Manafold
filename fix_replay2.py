with open('python/src/mtgml/replay.py', 'rb') as f:
    data = f.read()

old = b'def validate(self) -> None:\n        if self.randomness.contract_id != "mtgml.rng.v1":\n            raise WireError("semantic.replay_manifest", "unsupported RNG contract in replay")'

new = b'def validate(self) -> None:\n        if self.randomness.contract_id != "mtgml.rng.v1":\n            raise WireError("semantic.replay_manifest", "unsupported RNG contract in replay")\n        if not self.decks:\n            raise WireError("semantic.replay_manifest", "decks must not be empty")\n        seen_players = set()\n        for deck in self.decks:\n            if deck.player in seen_players:\n                raise WireError("semantic.replay_manifest", "duplicate player in decks")\n            seen_players.add(deck.player)\n            if not deck.deck_id:\n                raise WireError("semantic.replay_manifest", "deck_id must not be empty")\n        if self.schemas.replay_step != "replay-step.v2":\n            raise WireError("semantic.replay_manifest", "replay_step must be replay-step.v2")'

if old in data:
    data = data.replace(old, new)
    with open('python/src/mtgml/replay.py', 'wb') as f:
        f.write(data)
    print('Fixed!')
else:
    print('Old pattern not found!')