# Information Noninterference Testing

**Status:** accepted proof strategy

## Paired-state method

Construct two valid authoritative states that differ only in information unavailable to perspective `P`. For `P`, require byte-identical:

- observation and information-state wire payloads;
- visible decision constraints, candidate ordering, IDs, and semantic keys;
- observed events and visible sequence;
- sanitized errors;
- trajectory semantic fields;
- protocol metadata not explicitly allowed to differ.

## Typical hidden differences

- opponent hand identity with equal public counts;
- unseen library order;
- private looked-at cards;
- hidden face-down identities;
- sampled determinization choices;
- trusted RNG stream name/counter/root seed;
- authoritative object allocation history not exposed through opaque IDs.

## Lifecycle tests

Test knowledge gain, retained identity, invalidation after shuffle/randomization, new opaque identity after indistinguishability, checkpoint/restore continuity, and fork parity.

## Timing

Wall-clock timing is excluded from semantic bytes. Side-channel-resistant deployment is a broader systems problem, but the engine must not intentionally expose hidden-state-dependent counts or identifiers.
