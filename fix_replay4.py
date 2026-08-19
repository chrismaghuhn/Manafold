with open('python/src/mtgml/replay.py', 'rb') as f:
    data = f.read()

old = b'def to_wire(self) -> dict[str, object]:\n        self.validate()\n        return {\n            "deck_id": require_nonempty(self.deck_id, "deck_id"),\n            "digest": require_digest(self.digest),\n            "player": uint_wire(self.player),\n        }'

new = b'def to_wire(self) -> dict[str, object]:\n        return {\n            "deck_id": require_nonempty(self.deck_id, "deck_id"),\n            "digest": require_digest(self.digest),\n            "player": uint_wire(self.player),\n        }'

if old in data:
    data = data.replace(old, new)
    with open('python/src/mtgml/replay.py', 'wb') as f:
        f.write(data)
    print('Fixed!')
else:
    print('Old pattern not found!')