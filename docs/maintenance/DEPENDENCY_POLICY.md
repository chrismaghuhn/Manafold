# Dependency Policy

**Status:** accepted dependency policy  
**Stability:** normative


Prefer standard library and small focused dependencies in semantic hot paths.
Record purpose, trust boundary, and replacement cost. Lock dependencies; do not
auto-merge updates. Parser, codec, FFI, network, and source-fetch dependencies
are untrusted-input boundaries. Avoid dependencies that embed rules/card data or
global mutable state. Benchmark representation/hot-path changes.

Foundation CI may use reviewed vendor major tags; certified releases pin GitHub
Actions to reviewed full commit SHAs and use Dependabot for proposed updates.


The exact distinction between direct development pins, transitive locks, the reference interpreter/compiler, and public release build images is defined in [`TOOLCHAIN_POLICY.md`](TOOLCHAIN_POLICY.md). A filename containing `lock` is not by itself evidence of a fully resolved or hash-locked environment.
