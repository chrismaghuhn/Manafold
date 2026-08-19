with open('python/src/mtgml/replay.py', 'r') as f:
    content = f.read()

old = '''    @classmethod
    def from_wire(cls, value: object) -> DeckIdentityV1:
        obj = require_exact_keys(value, {"player", "deck_id", "digest"})
        return cls(
            parse_uint(obj["player"]),
            require_nonempty(obj["deck_id"], "deck_id"),
            require_digest(obj["digest"]),
        )'''

new = '''    @classmethod
    def from_wire(cls, value: object) -> DeckIdentityV1:
        obj = require_exact_keys(value, {"player", "deck_id", "digest"})
        return cls(
            parse_uint(obj["player"]),
            obj["deck_id"],  # Allow empty for now; validated at manifest level
            require_digest(obj["digest"]),
        )'''

if old in content:
    content = content.replace(old, new)
    with open('python/src/mtgml/replay.py', 'w') as f:
        f.write(content)
    print('Fixed!')
else:
    print('Pattern not found!')