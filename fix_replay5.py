with open('python/src/mtgml/replay.py', 'rb') as f:
    data = f.read()

# Remove self.validate() from classes that don't have validate method
classes_to_fix = [
    (b'class KernelIdentityV1', b'def to_wire(self) -> dict[str, object]:\n        self.validate()\n        return {'),
    (b'class ReplaySchemaVersionsV1', b'def to_wire(self) -> dict[str, object]:\n        self.validate()\n        return {'),
    (b'class RandomnessIdentityV1', b'def to_wire(self) -> dict[str, object]:\n        self.validate()\n        return {'),
    (b'class DeckIdentityV1', b'def to_wire(self) -> dict[str, object]:\n        self.validate()\n        return {'),
    (b'class ReplayStepV1', b'def to_wire(self) -> dict[str, object]:\n        self.validate()\n        return {'),
    (b'class RandomnessIdentityV2', b'def to_wire(self) -> dict[str, object]:\n        self.validate()\n        return {'),
    (b'class ReplayStepV2', b'def to_wire(self) -> dict[str, object]:\n        self.validate()\n        return {'),
]

for class_name, old_pattern in classes_to_fix:
    idx = data.find(class_name)
    if idx != -1:
        idx2 = data.find(old_pattern, idx)
        if idx2 != -1:
            new_pattern = old_pattern.replace(b'self.validate()\n        ', b'')
            data = data[:idx2] + new_pattern + data[idx2+len(old_pattern):]
            print(f'Fixed {class_name.decode()}')
        else:
            print(f'Pattern not found for {class_name.decode()}')

with open('python/src/mtgml/replay.py', 'wb') as f:
    f.write(data)
print('Done!')