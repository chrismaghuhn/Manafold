with open('python/src/mtgml/replay.py', 'rb') as f:
    data = f.read()

old = b'def to_wire(self) -> dict[str, object]:\n        return {'

new = b'def to_wire(self) -> dict[str, object]:\n        self.validate()\n        return {'

if old in data:
    data = data.replace(old, new)
    with open('python/src/mtgml/replay.py', 'wb') as f:
        f.write(data)
    print('Fixed!')
else:
    print('Old pattern not found!')