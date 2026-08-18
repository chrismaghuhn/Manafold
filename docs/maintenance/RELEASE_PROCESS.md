# Release Process

**Status:** accepted process

## 1. Name the release and claim

Identify release owner, freeze level, capability bundle if any, backend, and exact support/non-support claim.

## 2. Freeze identities

Pin source commit, dependencies/lockfiles, toolchain/build image, rules/format/Oracle/ruling snapshots, schemas, digest versions, deck/bundle manifests, RNG identity, benchmark scenarios, and reference hardware.

## 3. Run clean verification

From a clean checkout or source archive, run repository, Rust lexical structure, documentation, maintainer-artifact, Rust, Python, schema/fixture, conformance, noninterference, replay, fuzz/soak, security/dependency, and benchmark gates appropriate to the claim.

Generated verification data must be written outside the archived source set. The runner places an ownership marker in its output directory and refuses to recursively replace an existing unmarked directory. The default is:

```text
dist/verification/
├── logs/
├── verification-results.json
├── FOUNDATION_VERIFICATION.md
├── FOUNDATION_BLOCKERS.md
└── v0.2.2-status.json
```

## 4. Generate evidence

Generate—not manually edit—verification status, blockers, capability census, certification, benchmark, compatibility, provenance, and checksums. Preserve raw logs and failed results. Missing tools remain `NOT_RUN`.

## 5. Review contradictions and limitations

Resolve all contract contradictions. List explicit exclusions, revoked capabilities, unrun optional gates, and known limitations. A mandatory `NOT_RUN` blocks the requested release level.

## 6. Archive last

The deterministic source-archive gate is the final source-observing gate. `dist/`, logs, and generated reports are excluded from the archive. After the final archive gate, no archived file may be changed. Publish the source ZIP, SHA-256 sidecar, and adjacent verification evidence as separate artifacts.

For public releases, add attestation/signing when OD-021 is resolved.

A release cannot be certified if any reachable choice is automatically completed, any capability is missing, a reachable native executor is omitted from closure, or any numerical gate lacks pinned evidence.
