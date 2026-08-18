# Benchmarks

Benchmarks are committed workloads, not isolated headline numbers. Every result
must identify engine commit, capability bundle, snapshots, compiler and profile,
hardware manifest, warm-up, sample count, environment count, and raw output.

Required metrics eventually include:

- accepted transitions per second;
- decision points per second;
- legal-candidate generation latency;
- observation projection latency;
- checkpoint, restore, and fork latency;
- bytes allocated per step and per live environment;
- rule events per second;
- inference wait fraction for ML-integrated runs.

`results/` is empty by design. Do not commit a summarized claim without the raw
machine-readable result and the exact scenario used to produce it.
