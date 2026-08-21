# mtgml-persistence

Rules-neutral trusted persistence primitives for the M2 V3 semantic digest
contracts. This crate owns the restricted canonical CBOR profile, the V1
digest envelope, and the single `CheckpointDigestV3` calculation. It does not
own `EngineState`, checkpoints, replay execution, or public wire contracts.
