# ADR 0036: Trusted diagnostic sidecar and transition-centered debugging

- **Status:** accepted
- **Date:** 2026-08-19
- **Owners:** architecture maintainers; observability maintainers
- **Supersedes:** none
- **Superseded by:** none

## Context

Manafold is headless and must diagnose state, transition, replay, checkpoint, decision, RNG, and information-flow defects without weakening its capability or hidden-information boundaries. Privileged diagnostics are useful only if they remain observationally inert, reproducible, and clearly separate from authoritative semantics and player/model APIs.

Formatted logs, raw Rust serialization, GUI-side interpretation, or privileged methods on `PlayerEndpoint` would either create an unstable diagnostic authority or risk introducing a second rules/information surface.

## Decision

Manafold adopts the detailed contract in [`../DEBUG_ARCHITECTURE_CONTRACT.md`](../DEBUG_ARCHITECTURE_CONTRACT.md) and fixes these architectural choices:

1. The primary causal debugging unit is a transition diagnostic: before identity, current request/candidates, submitted response, validation result, accepted transition product or rejection, and after identity.
2. The portable reproduction unit is a complete deterministic start point—normally a complete checkpoint or declared initial fixture—plus an ordered replay/response segment and all required build/content/schema/RNG identities.
3. `StateDelta` remains the authoritative exact transition artifact. `DebugDiff` is a separate, read-only diagnostic comparison and is never executable as a patch.
4. Trusted diagnostics live below authoritative crates in dependency direction, conceptually through a dedicated diagnostics crate and one `manafold-dev` developer binary. Authoritative/player crates do not depend on developer diagnostics.
5. No privileged diagnostic type or command is exposed through `PlayerEndpoint`, public player wire formats, Python/model-facing APIs, or published trajectories.
6. Diagnostic artifacts, tracing, timing, and renderers are not replay authority and may not alter state, RNG, allocators, ordering, candidates, events, digests, replay, projections, or status.
7. Persisted machine-readable diagnostic/reproduction artifacts are explicitly versioned and sensitivity-classified. Human text/HTML layout is not compatibility-stable.
8. Root seed material is redacted by default and requires explicit secret-capability output. Exact trusted comparison happens before redaction; rendering/sinks enforce sensitivity separately.
9. Backward navigation and time travel use complete checkpoint restore followed by deterministic forward replay. Manafold does not implement inverse rule mutation or reverse `StateDelta` execution.
10. A graphical game UI is not required. Any later visual debugger is a read-only renderer over diagnostic artifacts; static self-contained HTML is preferred before an interactive local application.

## Consequences

- One semantic path/diff substrate can serve state inspection, transition explanation, checkpoint/replay comparison, conformance, information-safety reports, failure bundles, and later HTML rendering.
- Trusted maintainers may inspect authoritative IDs and hidden state without expanding the player/model capability surface.
- Diagnostic persistence requires explicit internal/experimental schema identities and compatibility handling.
- Failure artifacts containing hidden state or seeds require restricted handling; safe public summaries are separate renderings.
- M1 does not wait for the entire future tooling stack. Diagnostic pieces are implemented only when their owning milestone makes them useful.
- No M1 acceptance gate becomes `PASS` by accepting this ADR.

## Alternatives considered

- **Privileged methods on `PlayerEndpoint`:** rejected because capability separation is a security boundary, not a UI convention.
- **Logs as replay/debug authority:** rejected because logging is incomplete, renderer-dependent, and may be disabled or filtered.
- **Raw `EngineState` serialization as the stable debug format:** rejected because internal serialization is not a compatibility contract.
- **Event sourcing instead of authoritative state:** rejected; events remain audit evidence while `EngineState` remains semantic authority.
- **Executable diagnostic diffs:** rejected because a debug comparison must not become an alternate mutation path.
- **Reverse mutation/time-travel engine:** rejected in favor of checkpoint restore plus forward replay.
- **GUI/TUI-first debugging:** rejected due to maintenance cost and risk of duplicated semantic interpretation.
- **Python semantic debugger:** rejected because Python must not become a second rules/state engine.

## Evidence and follow-up

The design research motivating this decision compared deterministic simulation, replay/time-travel, compiler diagnostics, fuzz failure persistence/minimization, differential execution, and structured tracing. Those research notes are decision evidence, not a normative repository contract.

Implementation is milestone-scoped. The earliest useful pieces are structured invariant diagnostics, deterministic state views/diffs, transition/conformance explainers, and reproducible failure artifacts. Replay explanation waits for executable replay; richer minimization and visualization wait for demonstrated need.
