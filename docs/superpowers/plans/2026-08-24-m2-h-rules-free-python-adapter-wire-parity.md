# M2.H Implementation Plan — Rules-Free Python Semantic Adapter and Rust/Python Wire Parity

> **Status:** PLAN APPROVED — IMPLEMENTATION GO (Revision R4.1). External verdict: architecture APPROVED; GO effective with exactly the three R4.1 amendments below, which are incorporated. No branch created and no code written by this document; implementation starts from current `master`.
> Awaiting external review verdict before any implementation GO.
>
> **Revision history:**
> - R1 (2026-08-24): initial plan. External review: BLOCKER 3 / MAJOR 3 / MINOR 2 — NEEDS REVISION.
> - R2 (2026-08-24): all eight R1 findings addressed (twin oracle, shape-only submission encoder,
>   player-surface closure, panic termination, minimal authority, digest-input kept, no
>   requestless twin, consistent close/token semantics). External review: BLOCKER 3 /
>   MAJOR 4 / MINOR 2 — NEEDS R3.
> - R3 (2026-08-24): all nine R2 findings addressed:
>   - BLOCKER 1 → registry drift check replaced by an explicit **set relation**
>     (`COMMON_NAMED_CONTRACTS` + pinned `PYTHON_MECHANICAL_ONLY` exception) instead of
>     impossible three-way equality — consistent with keeping the digest-input entry.
>   - BLOCKER 2 → **direct-dependency allowlist ≠ transitive closure**; `mtgml-replay`
>     admitted DIRECTLY for configuration identity types only (`SyntheticM1EnvironmentConfig`
>     fields); tool-source privilege scan forbids replay *operations*
>     (`AuthoritativeReplay*`, `ReplayRecorder*`, `ReplayStep*`, checkpoint/restore/fork/
>     export/execute_*); transitive BFS forbidden-set claim dropped (could never pass).
>   - BLOCKER 3 → `episode_closed` removed from the Gate-B runtime matrix (unreachable:
>     the current synthetic kernel always emits `EpisodeStatus::Running` on completion);
>     its wire shapes remain Gate-A fixture parity; M2.D owns the semantic behavior;
>     **no status-injection backchannel**.
>   - MAJOR (dispatcher) → `selftest_roundtrip` **dropped** (`decode_named` is private;
>     zero production-wire changes). Cross-language authority = constructive encoding of
>     both Python and Rust against the SAME checked-in shared fixtures.
>   - MAJOR (co-drift) → new **below-JSONL handler transparency tests**: for every player
>     operation, endpoint result → `encode_canonical()` must equal the handler's emitted
>     payload bytes, so a shared adapter transformation bug cannot pass twin parity.
>   - MAJOR (raw bytes) → envelope payloads switched to **base64** (`*_wire_b64`),
>     confining base64 to the temporary JSONL shell and making the true `&[u8]` boundary
>     testable down to invalid UTF-8 / NUL / truncated multibyte / arbitrary garbage.
>   - MAJOR (field drift) → per-contract **schema-shape identity** (canonical structural
>     digest over required/optional properties, discriminators, `$ref` closure) pinned in
>     the manifest; any DTO field change fails the gate; optional fields require explicit
>     presence-AND-absence coverage.
>   - MINOR (trusted key) → key exists ONLY in the child process `env={...}`, never in the
>     pytest process `os.environ`; `AdapterPlayerClient` receives a restricted player
>     transport, never a generic send-command handle.
>   - MINOR (metadata) → all determinism/parity comparisons compare payload bytes only and
>     explicitly exclude transport metadata (request ids, tokens, frame structure).
> - R4 (2026-08-24): final narrow revision. External review of R3: BLOCKER 0 / MAJOR 2 /
>   MINOR 3 — "one final narrow revision", GO expected once these are incorporated. All
>   five findings addressed:
>   - MAJOR (count semantics) → two distinct counters defined (`endpoint_submit_calls`,
>     `kernel_transition_attempts`); `unavailable_decision` corrected to 1 endpoint
>     submit / 0 kernel transitions (layer-A decode succeeds, the endpoint IS called,
>     only the kernel transition is zero); malformed stays 0 / 0 (§H.5, §I).
>   - MAJOR (field drift) → Rust CONSTRUCTIVE PRODUCER tests added for every gate-owned
>     DTO: explicit struct literals (never `..Default::default()`) encoded and compared
>     against fixture bytes — adding/removing/retyping a Rust DTO field breaks
>     compilation until consciously re-reviewed. Four independent drift authorities
>     now: Rust constructive + Python constructive + SchemaContractDigest + fixture
>     bytes (§G.1, §Q).
>   - MINOR (cardinality) → `invalid_cardinality` added to the reachable Gate-B runtime
>     rejection classes (below-minimum `SelectMany`; membership/uniqueness precede
>     cardinality, so only the below-min arm fires) (§H.5, §I).
>   - MINOR (twin safety) → accepted-submit handler transparency never consumes an
>     accepted state twice: single-shot spy (step returned by EXACTLY ONE handler-driven
>     submit vs the payload bytes emitted by that same call) or twin fresh backends;
>     same-instance transparency reserved for reads and rejected submits (§H preamble).
>   - MINOR (naming) → digest renamed **SchemaContractDigest** with an explicit semantic
>     coverage definition: EVERY validation-relevant keyword reachable through the
>     closed `$ref` closure (types, required/optional, enum/oneOf, bounds, patterns,
>     additionalProperties, refs); excludes ONLY non-semantic annotations
>     (title/description/comments) (§G.1, §Q).
> - R4.1 (2026-08-24): final pre-implementation amendment. External review of R4:
>   BLOCKER 0 / MAJOR 2 / MINOR 1 — architecture APPROVED; **IMPLEMENTATION GO**
>   effective upon incorporating exactly the following corrections (no further plan
>   review required):
>   - MAJOR (no kernel counter) → `kernel_transition_attempts` REMOVED as an
>     executable/dynamic counter claim. There is deliberately NO kernel instrumentation
>     seam and NO new production test hook: M2.H dynamically proves ONLY
>     `endpoint_submit_calls` (malformed/envelope failures = 0; every typed submission
>     including `unavailable_decision` = exactly 1, via the counting decorator around
>     the REAL endpoint — the established M2.G seam pattern). Typed-rejection rows are
>     proven by real-endpoint-once + player-visible nonmutation + cross-twin parity;
>     complete authoritative/replay nonmutation and pre-kernel rejection ownership
>     remain the already-accepted M2.D/M2.G evidence (§H item 5, §I).
>   - MAJOR (error model) → a panic while servicing a valid PLAYER command NEVER
>     introduces a fourth player-visible error class: best-effort map to the frozen
>     layer-C `service_unavailable` surface, then terminate immediately; if emission
>     is unsafe, terminate and let the client observe transport closure — never any
>     trusted detail. `internal_error` exists only for trusted setup/orchestration
>     command failures and is unobservable through `AdapterPlayerClient` (§D, §I).
>   - MINOR (typed submit) → `AdapterPlayerClient.submit` stays protocol-exact typed
>     (`DecisionResponseV2 -> PlayerStepV2`, identical to the `PlayerClient` protocol).
>     Semantically-invalid instances are constructible directly as dataclasses (only
>     `validate()`/`to_wire()` judge), so the shape-only encoder transports them
>     verbatim without widening the public API; malformed/raw-byte tests use the
>     package-private `RestrictedPlayerTransport._submit_wire_bytes(token, bytes)`
>     seam — ONE player operation, ZERO trusted commands (§E, §J H.3/H.6).
>
> **Externally resolved decision points (reviewer verdicts on R2/R3):** trusted-key model
> ACCEPTED with the child-env restriction above; seed-pair subset ACCEPTED as sufficient
> above the M2.G-proven Rust boundary; digest-input disposition ACCEPTED (keep, model as
> explicit exception).
>
> **Baseline (verified 2026-08-24):** `master` @ `e250f6e06a1c1af65a53cd7b1e986fbcf1958644`
> (= PR #73 merge commit, M2.G), clean working tree.
> **Issue:** https://github.com/chrismaghuhn/Manafold/issues/55
> **Suggested implementation branch:** `chris/m2-h-python-adapter`

**Goal:** Prove, with exact executable evidence, that (A) every current M2 player-facing public DTO has mechanically equivalent Rust/Python canonical representation and rejection behavior (`M2_RUST_PYTHON_PLAYER_WIRE_PARITY`), and (B) a rules-free Python consumer can drive the real Rust perspective-safe M2 environment through a temporary non-published subprocess adapter without acquiring semantic authority (`M2_RULES_FREE_PYTHON_ADAPTER_PARITY`). OD-009 remains open.

**Architecture:** Rust remains the sole semantic authority. A temporary workspace-member tool binary (`tools/m2-semantic-adapter`) forwards canonical player DTO raw bytes verbatim between a Python test client and the real `PlayerEndpoint` boundary via JSONL-over-stdio with base64 payload confinement; trusted orchestration commands are capability-separated from token-scoped player commands; parity oracles use lockstep twin environments backed by below-JSONL handler-transparency proofs. Python gains an experimental internal `_m2_adapter` package (including a shape-only submission encoder so Python never pre-judges layer-B semantics) plus shared-fixture gap closure and static rules-free guards on both the Python and the Rust-adapter side.

---

## A. Verified repository baseline

```text
master SHA:            e250f6e06a1c1af65a53cd7b1e986fbcf1958644 (branch: master, working tree CLEAN)
                       = exactly the expected head; no drift to report
Issue #55:             OPEN — owns exactly M2_RUST_PYTHON_PLAYER_WIRE_PARITY
                       + M2_RULES_FREE_PYTHON_ADAPTER_PARITY
M2.G merge:            PR #73 MERGED 2026-08-24T17:09:18Z, mergeCommit = e250f6e (current head)
Python surface:        mtgml 0.2.2, src layout, ZERO runtime deps, strict mypy/ruff;
                       18 registered wire contracts (wire.py:_DECODERS) + an additional
                       information-state-digest-input.v2 entry (KEPT; modeled as the pinned
                       PYTHON_MECHANICAL_ONLY exception, §G.6);
                       full encode/decode for the whole player family; NOTE: DecisionAnswerV2
                       .to_wire() runs full semantic validate() (ascending select_many, unique
                       order ids) — this is why the transport needs a separate shape-only
                       submission encoder (§E);
                       mechanical digest recomputation (info-state v2, checkpoint v3);
                       PlayerClient protocol only (player_client.py), NO implementation,
                       NO subprocess/FFI/Rust interop anywhere in python/src
Rust player surface:   PlayerEndpoint trait (endpoint.rs:17–26): perspective / observation /
                       information_state / visible_decision / submit; submit returns
                       Ok(PlayerStepV2) for layer-B typed rejections,
                       Err(PlayerEndpointError::ServiceUnavailable) is the ONLY Err variant;
                       TrustedEnvironmentController (controller.rs:49–104): new/bind_player/
                       checkpoint/restore/fork/export_replay/execute_trusted_response/
                       execute_replay_from_checkpoint — no "reset": a new episode =
                       fresh backend + fresh controller; SyntheticM1EnvironmentConfig embeds
                       mtgml-replay identity types (KernelIdentityV1, ReplaySchemaVersionsV1,
                       DeckIdentityV1) — hence the direct-dependency allowance in §D/§L;
                       the layer-A/B seam already exists and is reserved for M2.H:
                       crates/mtgml-environment/src/boundary.rs::submit_response_bytes ->
                       PlayerBoundaryError::{Wire(MalformedResponse), Service(ServiceUnavailable)};
                       boundary.rs documents that decision_response_v2::decode_submission()
                       performs shape/canonical/schema-only validation and deliberately skips
                       response-local semantic checks — the endpoint owns those;
                       NOTE: mtgml_wire::decode_named(contract, bytes) is PRIVATE — no third
                       dispatcher will be built on top of it (§G.3)
existing parity:       shared corpora: 21 golden / 36 negative fixtures across 14 contracts,
                       consumed byte-exactly by both languages (mtgml-wire lib.rs:487–495;
                       python/tests/test_wire_contracts.py); schema-parity pytest;
                       persistence CBOR corpora separate;
                       synthetic-kernel fact (verified): the accepted-product builder always
                       emits EpisodeStatus::Running — a completed four-family chain removes
                       the continuation and clears pending_decision but never terminates the
                       episode, so `episode_closed` is UNREACHABLE through the real path (§H.5)
```

Key closed vocabularies (verified): 9 player-submission codes (`stale_decision`, `unavailable_decision`, `invalid_answer`, `invalid_candidate`, `duplicate_assignment`, `invalid_cardinality`, `invalid_number`, `invalid_order`, `episode_closed`), 1 wire code (`malformed_response`), 1 service code (`service_unavailable`), 7 observed-event kinds, 5+5 episode reasons, 4 answer families (`select_one`, `select_many`, `order`, `choose_number`).

## B. Current-state findings

**Already exists and must be reused (not duplicated):**

- All lower-layer DTO/codec parity (M2.B/D/E), perspective projection, three-layer error surface, knowledge/event lifecycle (M2.E), synthetic-program soundness/completeness (M2.F), and all Rust-side noninterference machinery (M2.G: paired axes 01–09/07a/07b, mutants m1–m12, multi-endpoint isolation including `case_wrong_perspective_closed_surface`, counting-seam zero-submit proof `wire_boundary.rs`, checkpoint/fork/replay parity).
- The exact layer-A seam (`submit_response_bytes`) was deliberately left for M2.H.
- Public byte collectors in `mtgml_conformance::isolation::fingerprint` — Rust-side reference patterns; the adapter tool itself does NOT depend on conformance (§D/§L).
- Hardened gate-runner conventions from `run_m2_g_gates.py`.
- The `PlayerClient` protocol in Python — the seam for the client.

**Actually missing (M2.H ownership):**

1. No Rust process boundary at all (zero `std::process`/IPC anywhere in `crates/**` — verified).
2. No concrete `PlayerClient` implementation in Python; no transport.
3. Golden-corpus gaps: `decision-response.v2` has only `select_one`; no goldens for `select_many`/`choose_number`/`order` answers or non-ChooseOne request shapes; `episode-status.v1` has only `running`; observed-event v2 covers 1 of 7 kinds; no PlayerStepV2 with non-empty events / terminal status; **no unknown-field negatives for any V2 DTO**; no byte-level noncanonical negative for V2; no schema-version-mismatch negative for the player family.
4. Real parity gap (C4): Python validates `ObservedEventEnvelopeV2` inner events shallowly while Rust is deeply typed → Rust rejects corruptions Python accepts. Must close.
5. Registry asymmetry: `information-state-digest-input.v2` stands in Python's `_DECODERS` but not in Rust's `decode_named`, with no standalone schema/fixtures. Disposition (externally accepted): **keep the Python entry**; the runner models it as an explicit pinned exception in the registry relation (§G.6) rather than deleting API behavior or forcing fake equality. Promotion to a standalone named contract remains a deferred option requiring its own coherent cross-layer pass.
6. No `run_m2_h_gates.py`; no PR Fast entry; no rules-free static guards; no fail-closed inventory preventing a newly added public player DTO/DTO-field/variant from escaping parity coverage (§G defines the mechanism).

No lower-level defect found. M2.G is not rerun or redesigned.

## C. Authority model

```text
TRUSTED TEST ORCHESTRATION (Python pytest harness)
  knows: root_seed(s), all tokens, trusted session key(s) (child-env only); may
         reset/bind/shutdown, drive the trusted-direct twin, compare payload bytes
        │ spawns ADAPTER PROCESSES (one per environment; twins = two processes),
        │ speaks JSONL per process, env-scoped trusted key
        ▼
TEMPORARY RUST ADAPTER (tools/m2-semantic-adapter, non-published)   [×N instances]
  owns: routing state ONLY (token registry, generation epoch). No game state,
        continuation, RNG, authoritative anything. Forwards canonical RAW BYTES VERBATIM
        (base64 confined to the temporary JSONL envelope, §D).
  trusted commands (require env-provided key): reset_synthetic, bind_player, direct_call,
        shutdown
  player commands (require token): observation, information_state, visible_decision, submit
  direct_call resolves ONLY already-bound PlayerEndpointHandles and invokes the SAME
  PlayerEndpoint trait methods the token path uses (no controller shortcut)
        │ in-process function calls
        ▼
TRUSTEDENVIRONMENTCONTROLLER → PERSPECTIVE-BOUND PLAYERENDPOINT HANDLES
  (Arc<Mutex<backend>>; handle permanently bound to one perspective, ADR-0020)
        ▼
AUTHORITATIVE RUST ENVIRONMENT (sole semantic authority: legality, RNG,
 continuations, mutation, events, digests)

PYTHON PLAYERCLIENT (mtgml._m2_adapter.AdapterPlayerClient)
  sees: its own token + its perspective's canonical player bytes + closed error codes.
  holds: a RESTRICTED player transport (four player ops), never a generic
         send(command) handle (MINOR fix).
  CANNOT: invoke trusted commands, rebind its perspective, read other players' bytes,
        pre-judge layer-B validity (shape-only submission encoder, §E).
```

Who sees what: the orchestrator sees everything player-visible for all perspectives across all twin environments plus setup inputs. Each adapter instance holds semantic state internally but emits only envelope + authorized bytes. A `PlayerClient` holder sees only its own token's canonical views. Nobody receives seeds/cursors/trusted IDs.

**Twin discipline:** an accepted submit advances revision/pending-decision/continuation, so the SAME instance can never serve as its own accepted-parity oracle. All accepted-transition comparisons run between two identically-reset environments driven in lockstep. Same-instance before/after assertions are used only where nonmutation is the property under test (typed rejections, malformed wire). **Co-drift defense:** because both routes share the adapter handler, handler output is additionally proven byte-transparent against the endpoint directly, below the JSONL layer (§H preamble, §J H.2).

## D. Proposed adapter architecture

**Location: new workspace member `tools/m2-semantic-adapter/`** (library core + thin `main.rs`).

**Dependency closure (BLOCKER-2 fix — direct allowlist ≠ transitive closure):**

```text
DIRECT dependencies allowed:
  mtgml-environment      controller/endpoints/backend/config/boundary seam
  mtgml-wire             canonical codec + decode_submission
  mtgml-decision         DecisionResponseV2 / PlayerDecisionRequestV2 types
  mtgml-observation      step/info-state/error-code types
  mtgml-model            PlayerId etc.
  mtgml-random           RootSeed256 construction for reset (only if the public API
                         requires it)
  mtgml-replay           CONFIGURATION IDENTITY TYPES ONLY (KernelIdentityV1,
                         ReplaySchemaVersionsV1, DeckIdentityV1 …) required to fill
                         SyntheticM1EnvironmentConfig; NO replay functionality used
FORBIDDEN direct: mtgml-state · mtgml-rules · mtgml-persistence · mtgml-conformance
Transitive closure: NOT asserted as a forbidden set — mtgml-environment itself transitively
  depends on state/random/rules/replay/persistence, so a transitive forbidden-set BFS could
  never pass. Asserted instead: (a) nothing in the workspace depends on the tool;
  (b) the tool does not depend on mtgml-conformance; (c) the tool-source privilege scan
  below forbids privileged OPERATIONS regardless of which crate provides the types.
```

Tool-source privilege scan (mechanical, H.6): forbids any call/reference in `tools/m2-semantic-adapter/src/**` to `checkpoint`, `restore`, `fork`, `export_replay`, `execute_trusted_response`, `execute_replay_from_checkpoint`, `AuthoritativeReplay*`, `ReplayRecorder*`, `ReplayStep*` — the tool owns configuration identity types but performs no replay/checkpoint operations, so no later "practical test helper" can introduce a privileged backchannel unnoticed.

As a workspace member the tool inherits fmt/clippy/test coverage from integration profiles and PR Fast. Set `publish = false`; placement under `tools/` signals unpublished test infrastructure (API_LIFECYCLE).

**Process model:** one process ↔ one live environment session. Sequential re-resets within a process are allowed; twins are separate processes. `shutdown` (trusted) or stdin EOF → clean exit 0. No daemon, no listeners.

**Token model:** opaque 128-bit random tokens per binding from OS entropy (exact-pinned `getrandom`; deliberately not derived from `mtgml-random`; no clock use). **Tokens are never echoed in responses**; request-id alone correlates. Issued once by `bind_player`, thereafter inbound-only. Unknown/expired/foreign token → uniform `unknown_token`. Routing-lifetime policy:

```text
reset_synthetic (trusted, mid-session): destroys controller; ALL previous tokens become
  invalid → subsequent use = uniform unknown_token.
shutdown/close: process exits cleanly. NO post-close protocol error exists — subsequent
  writes hit a closed pipe → local transport-closed error. No process remains to answer.
```

**Trusted command capability:** `reset_synthetic`/`bind_player`/`direct_call`/`shutdown` require a per-launch random `trusted_key`. Wrong/missing key → uniform `unknown_command`. **Child-environment scoping (MINOR fix):** the key is passed exclusively via the spawned child's `env={…}` mapping — never injected into the pytest process `os.environ`; a guard test enforces this, and `AdapterPlayerClient` structurally receives only a restricted player transport (no generic command channel). Deliberately no crypto (routing capability for a local test adapter).

`direct_call` (trusted, oracle-only): `{op: observation|information_state|visible_decision|submit, player}` — resolves the already-bound `PlayerEndpointHandle` and invokes the SAME trait methods as the token route.

**Failure policy:**

```text
Controlled service failure (poisoned lock, endpoint Err(ServiceUnavailable)):
  → closed service_unavailable-class response; process MAY continue.
Unexpected panic while servicing a PLAYER command (catch_unwind):
  → NEVER invent a fourth player-visible error class. Best-effort emit the frozen
    layer-C service_unavailable surface, THEN TERMINATE the process immediately
    (nonzero exit). If even that response cannot safely be emitted: terminate;
    the client observes transport closure. No recover-and-continue; no trusted
    panic detail ever reaches player-facing output.
Trusted setup/orchestration command failures:
  → may use the adapter-internal internal_error code; UNOBSERVABLE through
    AdapterPlayerClient (the player API carries no generic command channel).
```

Service-redaction evidence uses a controlled test-only failing endpoint/backend seam, never induced panics. The player-visible error surface remains EXACTLY: `malformed_response` (wire), closed submission codes (semantic), `service_unavailable` (internal/service).

**Framing (MAJOR raw-byte fix):** JSON Lines over stdin/stdout, UTF-8, strictly one request → one response, request ids echoed for correlation. Normative payloads travel as **base64 strings** (`observation_wire_b64`, `information_state_wire_b64`, `visible_decision_wire_b64`, `step_wire_b64`, `response_wire_b64`) — base64 belongs ONLY to the temporary non-normative JSONL envelope, so the exact original `&[u8]` reaches the Rust canonical decoder unmodified, including deliberately invalid bytes:

```text
Python canonical bytes ─base64→ JSONL ─decode→ exact original &[u8] ─→ Rust boundary
```

This makes the layer-A boundary fully exercisable down to raw-byte corruptions (invalid UTF-8, embedded NUL, truncated multibyte, arbitrary garbage) that a JSON-string carrier could not transport losslessly. Closed envelope codes: `parse_error`, `unknown_command`, `invalid_params`, `unknown_token`, `oversized_input`, `internal_error` — where `internal_error` is RESERVED for trusted-command failures and is never player-observable. stdout carries only envelope lines; stderr carries trusted diagnostics only. Input size capped (fail closed).

Command sets:

```text
trusted:   reset_synthetic {players:["1","2"], root_seed_hex}   # new epoch; old tokens dead
           bind_player {player} -> {token}                      # once per token
           direct_call {op, player, response_wire_b64?}      # bound-handle route, oracle only
           shutdown {}                                          # clean process exit
player:    observation          {token} -> {observation_wire_b64}
           information_state    {token} -> {information_state_wire_b64}
           visible_decision     {token} -> {visible_decision_wire_b64|null}
           submit               {token, response_wire_b64}
                                        -> {step_wire_b64}            # Ok incl. typed rejection
                                        | ok:false envelope error code malformed_response|service_unavailable
```

Layer B arrives INSIDE the step (`submission.rejected`) per the existing Rust contract; `ok:false` boundary errors occur only for wire/service layers — mirroring `submit_response_bytes`.

## E. Python client architecture

New experimental, non-public subpackage **`python/src/mtgml/_m2_adapter/`** (leading underscore = internal/experimental; not in the public `__all__`):

```text
_m2_adapter/__init__.py   re-exports SyntheticEnvironmentClient, AdapterPlayerClient, AdapterError
_m2_adapter/protocol.py   envelope dataclasses + closed adapter-code set (mechanical only);
                          base64 payload confinement helpers
_m2_adapter/process.py    SubprocessTransport: spawn with CHILD-SCOPED env (trusted key only
                          in the child env mapping), line framing, req/resp pairing by
                          request id, timeouts, EOF/hang termination, binary located via
                          MTGML_M2_ADAPTER_BIN (fail closed if unset); post-shutdown writes
                          raise a local transport-closed error (no protocol round-trip)
_m2_adapter/submission.py encode_decision_response_submission_v2(payload) -> bytes
                          SHAPE-ONLY submission encoder — see below
_m2_adapter/client.py     SyntheticEnvironmentClient  (TRUSTED: reset_synthetic(players,
                          root_seed_hex), bind_player(player) -> AdapterPlayerClient,
                          shutdown; context manager)
                          AdapterPlayerClient (implements mtgml.PlayerClient protocol;
                          holds EXACTLY ONE token; restricted player transport ONLY —
                          no generic send/command method)
```

Stdlib only (still zero runtime deps).

**Shape-only submission encoder.** The accepted Rust boundary splits `decode_submission` (shape/canonical/schema-only) from endpoint-owned response-local semantics; Python mirrors that split on the producer side:

```text
MAY check:    closed answer-kind variant; exact per-variant key sets (required present,
              unknown rejected); scalar representations (u32 candidate_id, i64 value,
              decimal-string ids); schema identity string; canonical compact JSON output.
MUST NOT:     candidate membership, duplicate answer ids, ascending select_many, order
              uniqueness, cardinality vs request, numeric request bounds, decision/
              revision correspondence, anything request-relative.
```

It operates on plain wire-shape mappings (or directly on DTO field data) so tests can construct semantic-invalid payloads without triggering semantic validation.

**Transport input contract (MINOR fix, R4.1):** `AdapterPlayerClient.submit(response: DecisionResponseV2) -> PlayerStepV2` — EXACTLY the existing `PlayerClient` protocol signature; the public client API is NOT widened to arbitrary mappings. The client serializes the DTO through the shape-only submission encoder WITHOUT invoking semantic `validate()`/`to_wire()`. A semantically INVALID response needs no special typing: `DecisionResponseV2` is constructible directly as a dataclass (only `validate()`/`to_wire()` judge semantics), so typed-rejection rows build typed-but-semantically-invalid instances and the encoder transports them verbatim; bytes pass to the adapter untouched and Rust judges exactly once.

Raw/malformed byte tests use a PACKAGE-PRIVATE seam beneath the client:

```text
AdapterPlayerClient.submit(DecisionResponseV2)      # typed, protocol-exact
    ↓ shape-only submission encoding
RestrictedPlayerTransport._submit_wire_bytes(token, bytes)
    ↓ base64 JSONL
Rust submit_response_bytes()
```

`_submit_wire_bytes` carries ONLY the one player submit operation and ZERO trusted commands, so the lowest test layer can inject arbitrary bytes without any generic command channel on the client. An explicit Gate-B divergence test proves the split intentionally: a duplicate-id/unsorted typed instance passes the submission encoder, reaches Rust, and is rejected exactly once (`duplicate_assignment`/`invalid_order`), while full `to_wire()` rejects the same value locally (fixtures/Gate-A context only).

**Intentionally forbidden helpers (guard-enforced in H.6):** `legal_actions`, `best_action`, `auto_step`, `auto_answer`, `auto_complete_response`, `repair_response`, `resolve_candidates`, any deciding `step()`, and any generic command-send surface on the player client.

## F. Rust/Python wire inventory (player-facing M2 set)

Legend: ✅ present · ➖ n/a · ❌ missing.

| DTO/type | Rust codec | Py codec | Schema | Golden | Negative | Semantic validation | Gap / R3 disposition |
|---|---|---|---|---|---|---|---|
| `player-decision-request.v2` | ✅ | ✅ | ✅ | 1 (choose_one) | 0 | both (ordering rank tables, dense ids) | goldens for other request kinds; V2 negatives |
| `decision-response.v2` (+ 4 answer families) | ✅ | ✅ (full `to_wire` validates semantically — NOT on submit path) | ✅ | 1 (select_one) | 2 | both | goldens select_many/order/choose_number; more negatives; NEW shape-only submission encoder for transport |
| `observation-envelope.v1` | ✅ | ✅ | ✅ | 1 | 1 | both | — |
| synthetic observation payload `synthetic-m2-observation.v1` | ✅ (opaque pipe-UTF8) | ➖ base64+codec string only | ❌ (deliberately codec-identified, not JSON) | via observation golden | — | Rust-side producer | — (payload stays opaque to Python by design) |
| `information-state-envelope.v2` (+ `PlayerKnownObjectV1` active/retired, provenance) | ✅ | ✅ | ✅ | 1 (covers active+retired) | 4 | both + Python digest recomputation | sufficient; add 1 V2 unknown-field negative |
| `information-state-digest-input.v2` | ✅ nested, not in `decode_named` | ✅ nested **and** registry entry (**KEPT**) | ❌ | ❌ | ❌ | both (algorithmic owner: `mtgml-wire`; Python mechanically reproduces) | classified nested/mechanical-only; modeled as the pinned `PYTHON_MECHANICAL_ONLY` registry exception (§G.6); promotion deferred |
| `observed-event-envelope.v2` (7 kinds) | ✅ deep | ⚠️ shallow (kind only; payload opaque) | ✅ | 1 of 7 kinds | 1 | Rust deep / Python shallow | **C4 parity gap: deepen Python + kind-covering goldens/negatives** |
| `player-step.v2` (+ nested `PlayerStepSubmissionV1`, 9 codes) | ✅ | ✅ | ✅ | 2 (accepted, stale-rejected) | 8 | both | golden with events + terminal status + `episode_closed` submission code (**wire parity only** — see §H.5 reachability note); V2 unknown-field negative |
| `episode-status.v1` (running/terminal/truncated, 5+5 reasons) | ✅ | ✅ | ✅ generated | 1 (running) | 2 | both | terminal + truncated goldens (Gate A only) |
| wire error `malformed_response` / service `service_unavailable` | ✅ | ➖ (codes as constants) | ➖ | via negatives | ✅ (classes) | Rust boundary | Py adapter mirrors codes (new, experimental) |

Non-player public families (`replay-manifest.*`, `authoritative-replay.*`, V1 contracts) are already corpus-covered and are not player-facing; they remain full-regression evidence via the shared manifest loops, not H-completeness authority (§G).

## G. Gate A plan — `M2_RUST_PYTHON_PLAYER_WIRE_PARITY`

Gate definition (verbatim): *"requires byte-exact Rust/Python canonical fixtures for all public M2 DTOs and exact negative-corpus agreement."*

Evidence (all pytest + mtgml-wire nodes under the H runner):

1. **Completeness authority: player-surface closure manifest + SchemaContractDigest identities.**
   ```text
   REQUIRED_PLAYER_SURFACE (gate-owned, hand-reviewed manifest):
     Rust side:  exact PlayerEndpoint trait method set + return types (endpoint.rs),
                 PlayerBoundaryError variants/codes (boundary.rs),
                 PlayerEndpointError variants (endpoint.rs) — extracted mechanically.
     Python side: exact PlayerClient protocol signatures (player_client.py) and
                 AdapterPlayerClient public method sets (_m2_adapter/client.py).
   ↓ derivation (reviewed, explicit):
     required top-level wire contracts:
       ObservationEnvelopeV1, PlayerInformationStateV2, PlayerDecisionRequestV2,
       DecisionResponseV2, PlayerStepV2 (+ PlayerStepSubmissionV1 codes),
       EpisodeStatus (+reasons), ObservedEventEnvelopeV2 (+7 kinds),
       submission/wire/service error codes
     ↓ transitive closure (explicitly enumerated):
       CandidateIntent/DecisionSpec/VisibleCandidateV2/DecisionAnswerV2 families,
       PlayerKnownObjectV1 active/retired, provenance channels/causes/kinds,
       knowledge invalidation reasons, digest identity strings
   ↓ coverage obligations per entry:
       JSON Schema exists · Rust codec exists · Python codec exists ·
       golden coverage per variant class · negative coverage per rejection class
   ```
   Mechanical signature guards fail the gate on ANY signature change (new method, changed return type, renamed field).

   **SchemaContractDigest (MAJOR/MINOR field-drift fix, R4):** for every top-level contract, the runner computes a canonical **SchemaContractDigest** from the checked-in JSON Schema: a normalized serialization of EVERY validation-relevant keyword reachable through the contract's closed `$ref` closure — property names and types, required/optional partitioning, enum/oneOf variants, numeric bounds, string patterns, `additionalProperties`, nested refs — excluding ONLY non-semantic annotations (`title`, `description`, comments). Digests are pinned in the manifest; ANY semantic schema drift (including a lone changed `maximum` or `pattern`) changes the digest → gate FAILS until manifest and positive+negative coverage are updated coherently. For every optional field, coverage must include BOTH presence and absence forms (e.g., `next_decision` present/absent, nullable `known_definition`). A cheap consistency check additionally asserts every fixture's key usage stays within the schema properties.

   **Rust constructive producer tests (MAJOR field-drift fix, R4):** for every gate-owned player DTO, an mtgml-wire test node CONSTRUCTS the value with an explicit struct literal (never `..Default::default()`) and asserts `encode_canonical(&value)` == the checked-in fixture bytes. Adding, removing, or retyping a Rust DTO FIELD breaks compilation of this node until the DTO is consciously re-reviewed; enum additions remain guarded by the variant closure. The gate thus holds FOUR independent safeguards against field/type drift: Rust constructive DTO + Python constructive DTO (§G.3) + SchemaContractDigest + shared fixture byte identity — deliberately stronger than more source regex.

2. **Corpus loops as full regression:** for EVERY `wire/golden` entry: Rust decode→re-encode == bytes AND Python decode→re-encode == bytes (both exist today). Full-regression scope; deliberately NOT the completeness authority.
3. **Cross-language proof without a third dispatcher (MAJOR fix — `selftest_roundtrip` dropped):** `mtgml_wire::decode_named` is private and this plan commits to zero production-wire changes, so no dynamic contract dispatch is built. Instead both languages are proven against the ONE common checked-in byte authority:
   ```text
   Producer side (new pytest): for every REQUIRED golden class, CONSTRUCT the value from
     public domain data in Python → encode → byte-compare against the checked-in fixture.
     Negative classes: construct the corrupt form → expect the exact closed code.
   Consumer side (existing): Rust decodes the SAME fixture bytes → re-encode → identical;
     Python decodes the SAME fixture bytes → re-encode → identical.
   ```
4. **Exact negative-corpus agreement:** every negative manifest entry rejected by BOTH languages with the identical expected code (extended per gaps #3/#4, incl. raw-byte corruption classes delivered via the base64 envelope, §D).
5. **Close the gaps:** new goldens/negatives per table F; deepen Python `ObservedEventEnvelopeV2` to mirror Rust; update `test_m2_b_staging_fixtures` expectations coherently (historical baseline checks untouched); KEEP the digest-input Python entry (§F).
6. **Decoder-registry RELATION (BLOCKER-1 fix — replaces impossible three-way equality):**
   ```text
   COMMON_NAMED_CONTRACTS := exact set of Rust decode_named contract arms
   PYTHON_MECHANICAL_ONLY := {"information-state-digest-input.v2"}   # exact, pinned frozenset
   Drift node asserts:
     rust decode_named set          == COMMON_NAMED_CONTRACTS
     python _DECODERS set           == COMMON_NAMED_CONTRACTS ∪ PYTHON_MECHANICAL_ONLY
     validate_schemas.WIRE_MAPPING  == COMMON_NAMED_CONTRACTS
   Any addition/removal touching PYTHON_MECHANICAL_ONLY fails the gate until the
   exception is explicitly re-reviewed. This is a REGRESSION/drift node, not the
   completeness authority (that is §G.1), and it is TRUE on today's tree.
   ```
7. **Self-tests (§26):** mutate the surface manifest/signature fixture/SchemaContractDigest/registry exception/fixture file in a temp copy → runner must FAIL; changing a single validation-relevant schema keyword (e.g., a `maximum`) WITHOUT updating the pinned digest must FAIL; skip/xfail/deselect/dirty-tree/wrong-head must all FAIL; removing one required variant or one optional-field presence case must FAIL.

## H. Gate B plan — `M2_RULES_FREE_PYTHON_ADAPTER_PARITY`

Gate definition (verbatim): *"requires a temporary non-published Python consumer to drive the real Rust perspective-safe endpoint without legality/state/RNG/replay authority. It does not resolve OD-009."*

Everything below is **pytest-driven, Python issues every move** (real binaries, real `submit_response_bytes`, real endpoints), reusing the frozen synthetic program.

**Comparison hygiene (MINOR fix):** all byte-equality/determinism assertions compare **payload bytes only**; transport metadata — request ids, tokens, frame boundaries, frame ordering — is explicitly excluded from every comparison. Tokens never appear in any payload.

**Co-drift defense (MAJOR fix; accepted-submit twin safety tightened in R4):** twins share the adapter handler, so twin equality alone could pass a systematically wrong shared transformation. Therefore every player operation carries a **below-JSONL handler-transparency proof** (Rust unit tests in H.2):

```text
reads (observation / information_state / visible_decision Some+None):
    endpoint read → encode_canonical() == handler-emitted payload bytes
    (same instance permitted — reads are pure; projection purity is contract-owned)
rejected submits:
    endpoint.submit(response) → encode_canonical(step) == handler step_wire bytes
    (same instance permitted — rejections are defined nonmutating)
ACCEPTED submits (an accepted state is NEVER consumed twice):
    primary form: single-shot spy — the PlayerStep returned by EXACTLY ONE
      handler-driven submit call is compared against the payload bytes emitted
      by that same call;
    alternative form: two identical fresh backends, direct submit on A vs
      handler submit on B, canonical steps compared.
```

Evidence chain: Rust endpoint bytes → (exact transparency proof) → adapter handler bytes → (twin E2E proof) → Python. A shared transformation bug can no longer satisfy both twins.

**Lockstep twin architecture:** twin A drives every op via token commands; twin B replays the identical history via trusted `direct_call` (same bound handles, second route). Per stage asserted byte-equal across twins: decision bytes → response → step bytes → post-step views. Accepted transitions are NEVER compared against the same instance; rejections/malformed may additionally use same-instance before/after zero-mutation assertions.

Scenarios:

1. **Reset determinism:** twins' initial views byte-equal (payloads only, §hygiene). No seed ever returned through a player command.
2. **Perspective binding / wrong-perspective probe:** with P1 holding a visible request, assert P2 `visible_decision() == None`; submit P1-derived response under P2's token → `unavailable_decision`; zero mutation; no trusted detail. M2.G's indistinguishability proof stands at the Rust layer; M2.H proves the adapter/client layer adds no distinguishing channel. **No requestless-twin scenario backchannel.**
3. **Explicit decision chain (primary twin A):** all four families answered explicitly from publicly offered data — choice by test, judgment by Rust.
4. **Accepted parity (lockstep twins):** twin B replays twin A's identical response history via `direct_call`; per-stage payload bytes compared cross-process AND cross-route; grounded by the H.2 transparency proofs.
5. **Typed rejection parity — REACHABLE CLASSES ONLY, now EXACT (BLOCKER-3 fix; cardinality completed in R4):** `unavailable_decision` (foreign actor, item 2), `stale_decision`, `invalid_answer`, `invalid_candidate`, `duplicate_assignment`, `invalid_cardinality` (shape-valid `SelectMany` with FEWER than the request minimum — membership/uniqueness precede cardinality, so only the below-minimum arm fires on live requests, consistent with M2.G's finding), `invalid_number`, `invalid_order`. Count semantics (R4.1): every typed submission — rejection or wrong-actor — makes EXACTLY ONE endpoint submit (`endpoint_submit_calls == 1`: layer-A decode succeeded, the endpoint IS called; proven dynamically by the counting decorator around the REAL endpoint, the existing M2.G seam pattern). There is deliberately NO kernel-transition counter and NO production kernel instrumentation: M2.H proves player-visible NONMUTATION plus cross-twin parity for these rows, and relies on the already-accepted M2.D/M2.G evidence for complete authoritative/replay nonmutation and pre-kernel rejection ownership. The true zero-submit domain is malformed bytes alone. **`episode_closed` is REMOVED from the runtime matrix:** the current synthetic kernel always emits `EpisodeStatus::Running` on completion (continuation removed, pending decision cleared, revision advanced — never terminal), so no real-path episode can produce it, and building a status-injection/debug backchannel to force it would be scope creep. Its WIRE parity remains owned by Gate A (terminal/truncated status goldens; `episode_closed` submission-code step fixture), and its SEMANTIC behavior remains owned by existing M2.D regression evidence. If the kernel later terminates episodes natively, extending the H matrix is a one-row follow-up. Rejection rows are injected identically into both twins; zero mutation per-instance; lockstep continues afterward.
6. **Malformed wire boundary (raw bytes via base64 envelope):** corruption classes now include BOTH document-level defects (leading whitespace, key order, unknown field, wrong schema version, truncated JSON, u32 overflow) AND true byte-level defects the JSON-string carrier could not express (invalid UTF-8 sequences, embedded NUL, truncated multibyte, arbitrary garbage bytes). Each → envelope reports `malformed_response`, no step; zero mutation; zero semantic submits proven compositionally with the existing seam evidence (`wire_boundary.rs`) plus the H.2 counting-decorator proof through the adapter handler.
7. **Multi-endpoint isolation (primary environment):** both endpoints coexist; public/common facts agree, private facts diverge per perspective (marker search, G.3 style); query purity across interleaved read orders.
8. **Paired hidden variants (representative subset — externally accepted):** seed-pair proof (`"11"*32` vs `"22"*32`, independent processes → P1's complete public payload sequence byte-equal) plus items 4/6. Justification recorded: M2.G proved the ten hidden axes at the Rust boundary; M2.H proves its new layers add no leaks (seed-pair + isolation + wrong-actor + raw-byte/rejection + restart + transparency proofs suffice).
9. **Restart determinism:** shut down twin A's process; relaunch; identical inputs and response script → byte-identical concatenated public payload sequence.

## I. Negative/adversarial matrix

| Case | Layer | Public result | Endpoint submit calls | State mutation | Replay mutation | Player-visible mutation |
|---|---|---|---|---|---|---|
| Malformed/noncanonical `response_wire` (document-level 6 classes + raw-byte classes: invalid UTF-8, NUL, truncated multibyte, garbage) | A | `malformed_response`, no step | **0** (seam-proven) | none | none | none |
| Unknown schema/version in bytes | A | `malformed_response` | **0** | none | none | none |
| Typed reachable rejections: stale/candidate/cardinality (below-min)/number/duplicate/order/answer-family (incl. typed instances Python's full validator would reject) | B | `Ok(step)` with closed code | **exactly 1** — endpoint called once post-decode; player-visible nonmutation asserted; authoritative/replay nonmutation remains M2.D/M2.G evidence | none (player-visible) | none (per M2.D/M2.G) | step bytes only |
| Foreign-actor submission via other token | B | `unavailable_decision` (uniform) | **exactly 1** — layer-A decode succeeds, the endpoint IS called; the pending request is unavailable to this perspective | none | none | none |
| `episode_closed` | — | NOT exercised at runtime (unreachable via current synthetic path); wire parity via Gate A fixtures; semantics via M2.D regression | — | — | — | — |
| Token after mid-session reset | Envelope | uniform `unknown_token` | 0 | none | none | none |
| Use after `shutdown` | Transport | process exited; local transport-closed error (EOF/broken pipe) | n/a | n/a | n/a | n/a |
| Trusted command without key / unknown command | Envelope | uniform `unknown_command` | 0 | none | none | none |
| Malformed envelope line / oversized input | Envelope | `parse_error` / `oversized_input` | 0 | none | none | none |
| Unexpected panic while servicing a PLAYER command | C | best-effort closed `service_unavailable`, then **process terminates** (nonzero); if emission is unsafe, termination only and the client observes transport closure — NEVER a player-visible `internal_error`, never trusted detail | 1 (submit already happened) or n/a for reads | none observable | — | none |
| Controlled service failure (poisoned lock / failing seam) | C | `service_unavailable`, process MAY continue | — | none by design | — | none |
| Trusted setup/orchestration command failure (adapter-internal) | Trusted-only | adapter-internal failure possible — unobservable through `AdapterPlayerClient` | n/a | none | none | none |
| Accepted response (reference row) | B/C | `Ok(step)`, running status advanced | **1** | intended transition | recorder appends | step + new views |

Counter policy (R4.1): M2.H dynamically asserts ONE counter only — `endpoint_submit_calls` (invocations of `PlayerEndpoint::submit` after successful layer-A decode, counted by the counting decorator around the real endpoint): malformed and all envelope/wire failures = 0; EVERY typed submission including `unavailable_decision` = exactly 1. There is deliberately NO kernel-transition counter and NO production kernel instrumentation seam: state-unchanged assertions are player-visible properties, and complete authoritative/replay nonmutation plus pre-kernel rejection ownership remain the accepted M2.D/M2.G evidence. A test asserting "0 submits" for `unavailable_decision` would bypass the real endpoint path and is itself a defect.

No trusted error strings ever appear in player-facing assertions; diagnostics stay on stderr/logs; redaction evidenced via the controlled failing seam.

## J. Implementation slices

```text
H.1 Wire-inventory closure
  Goal: close table-F gaps; Python V2 event depth; staging-inventory coherence;
        KEEP digest-input Python entry. Includes Gate-A-only fixtures for terminal/
        truncated episode status and the episode_closed submission-code step shape.
  Files: wire/golden/*, wire/negative/*(+manifests), python/src/mtgml/observation.py,
         python/tests/*, crates/mtgml-wire (tests only)
  Positive: new family/variant goldens; constructive-encoding tests (§G.3).
  Negative: unknown-field V2, noncanonical V2, schema-version, deep V2 event corruption.
  Safety: mechanical; converges Python→Rust. Determinism: none touched.
  Review lens: cross-layer coherence (ADR-0019 atomicity). Stop: just check-fast green.

H.2 Rust adapter shell
  Goal: tools/m2-semantic-adapter (lib+thin main) with the DIRECT dependency allowlist
        of §D (incl. mtgml-replay CONFIG TYPES ONLY); framing with BASE64 payload
        confinement; token registry (getrandom, no echo); trusted-key child-env gating;
        reset (epoch invalidation)/bind/direct_call (bound-handle route)/shutdown;
        four player ops via submit_response_bytes; FAILURE POLICY (player-command panic ⇒
        best-effort service_unavailable then terminate; NO player-visible internal_error);
        HANDLER TRANSPARENCY TESTS for every op below JSONL (reads + rejected submits
        same-instance permitted; ACCEPTED submit via single-shot spy or twin fresh
        backends — an accepted state is never consumed twice), asserting the SINGLE
        dynamic counter endpoint_submit_calls (NO kernel instrumentation seam,
        NO new production hooks);
        stdout/stderr discipline. NO selftest_roundtrip subcommand (dropped).
  Positive tests: framing/base64 fidelity (byte-exact round-trip incl. invalid UTF-8
        payloads), request-id correlation without token echo, reset invalidation,
        EOF-after-shutdown, accepted-chain smoke, malformed classes ⇒ endpoint_submit_calls 0,
        typed rejections & wrong-actor ⇒ endpoint_submit_calls exactly 1 (counting
        decorator around the real endpoint),
        player-command panic ⇒ best-effort service_unavailable then terminated process
        (and no player-observable internal_error anywhere),
        failing-seam redaction with continued service,
        per-op transparency equalities
  Negative: all envelope codes; oversize; foreign/expired tokens
  Guards: cargo-metadata DIRECT-dependency allowlist (forbidden direct: state/rules/
        persistence/conformance; no depends-on-tool assertion workspace-wide); tool-source
        privilege scan (no checkpoint/restore/fork/export_replay/execute_*/
        AuthoritativeReplay*/ReplayRecorder*/ReplayStep*)
  Determinism: routing state non-authoritative; tokens never in payloads/responses
  Review lens: ADR-0010/0014/0020 fidelity; minimal authority. Stop: cargo +1.85.1 test/clippy green.

H.3 Python transport shell + submission encoder
  Goal: _m2_adapter package (process with CHILD-SCOPED trusted-key env, protocol with
        base64 confinement, submission, clients) with PROTOCOL-EXACT typed submit
        (DecisionResponseV2 -> PlayerStepV2) and a package-private
        RestrictedPlayerTransport._submit_wire_bytes seam (single player operation,
        zero trusted commands) for raw-byte tests + fake-transport unit tests +
        API-inventory test (exact typed signatures; absence of any generic send/command;
        untouched parent os.environ).
  Negative: timeout, child crash, malformed frame, binary unset, post-shutdown write → fail closed
  Stop: mypy/ruff/pytest green without the binary; encoder property tests prove the
        shape/semantic split matches decode_submission's contract.

H.4 Lockstep-twin semantic parity (Gate-B core)
  Twin infrastructure + scenarios 1, 3, 4, 5 (EXACTLY the eight reachable rejection
        classes incl. invalid_cardinality below-minimum; count semantics per §I),
        6 (raw-byte classes included), with payload-only comparison hygiene.
  Stop: all nodes green locally; no helpers/backchannels added (proven by H.6).

H.5 Isolation + paired seeds + restart (Gate-B completion)
  Scenarios 2, 7, 8, 9. Stop: green; twin/transparency oracle chain documented;
        no requestless-twin or scenario-injection backchannel exists (guard-checked).

H.6 Rules-free static guards (Python AND Rust-adapter side)
  Python: pyproject deps==[] assertion; import scan; symbol scan; API inventory
        introspection (protocol-exact typed method sets; the package-private
        _submit_wire_bytes seam named as the sole raw-byte entry point).
  Rust: tool-source privilege scan (§D); DIRECT-dependency allowlist enforcement;
        confirmation that direct_call resolves bound endpoint handles only;
        no scenario/status-injection command exists.
  Review lens: §21 — dependency + API + source + end-to-end proof, not grep alone.

H.7 Gate runner + CI + self-tests
  scripts/run_m2_h_gates.py (fresh file, G skeleton WITH fixes: per-node log indices,
        FAIL-dominant aggregator, per-node timeout→BLOCKED, structured startup errors);
        owns EXACTLY the two gates; REQUIRED_PLAYER_SURFACE manifest + signature
        extraction (COMPLETENESS AUTHORITY) + per-contract SchemaContractDigest
        identities (semantic field-drift authority) + Rust constructive producer
        nodes per gate-owned DTO + derived transitive variant closure;
        registry RELATION node (§G.6) as drift/regression; EXPECTED_EVIDENCE exact-set;
        dist/m2-h-verification + marker; python/tests/test_m2_h_gate_runner.py mutation
        matrix (signature drift, field drift, variant removal, registry-exception change,
        skip/xfail/dirty/wrong-head → FAIL); pr-fast.yml +6-line suffix.
```

Each slice independently reviewable; H.2/H.3 parallelizable after H.1; H.7 last.

## K. Expected file changes

**New:** `tools/m2-semantic-adapter/{Cargo.toml,src/main.rs,src/lib.rs,src/protocol.rs,src/tokens.rs,src/handlers.rs,…}` (NO selftest subcommand); `python/src/mtgml/_m2_adapter/{__init__,protocol,process,submission,client}.py`; `python/tests/test_m2_adapter_{unit,scenarios,isolation,guards}.py`; `scripts/run_m2_h_gates.py`; `python/tests/test_m2_h_gate_runner.py`; new goldens/negatives + manifest entries; root `Cargo.toml` member entry + Cargo.lock.

**Modified:** `python/src/mtgml/observation.py` (V2 event depth), `.github/workflows/pr-fast.yml` (+6), `python/tests/test_m2_b_staging_fixtures.py` (expectation list), optionally dedupe `test_schema_parity.py` mapping; `schemas/README.json` refresh ONLY if touched — otherwise left stale (pre-existing follow-up, §R).

**Explicitly untouched:** `python/src/mtgml/wire.py` (NO registry deletions; the digest-input entry stays), all production sources of `crates/mtgml-*` including `mtgml-wire` (`boundary.rs`/`endpoint.rs` consumed as-is; no visibility changes — the dropped selftest dispatcher was the only reason any would have been needed), M2.G isolation modules, `contracts/catalog/*` (no generator scope change → no new ADR), all V3 persistence semantics, normative docs.

## L. Dependency graph

```text
DIRECT dependencies of tools/m2-semantic-adapter (allowlist):
  mtgml-environment ← mtgml-wire ← {decision, observation, model}
  mtgml-random   (RootSeed256, if required by the public reset API)
  mtgml-replay   (CONFIGURATION IDENTITY TYPES ONLY: KernelIdentityV1 /
                  ReplaySchemaVersionsV1 / DeckIdentityV1 for
                  SyntheticM1EnvironmentConfig — NO replay functionality)
FORBIDDEN direct: mtgml-state · mtgml-rules · mtgml-persistence · mtgml-conformance
Transitive closure: deliberately NOT a forbidden-set assertion (environment itself is
  transitively linked to state/random/rules/replay/persistence — such a BFS can never
  pass); asserted instead: nothing depends on the tool · tool ↛ conformance ·
  tool-source operation-level privilege scan (§D)
python/mtgml (stdlib only) --spawns one subprocess per twin--> tools/m2-semantic-adapter
NO arrows: production crates → tool · tool → conformance · python → conformance · rust → python
```

OD-009 untouched (tool unpublished, `publish = false`); enforced mechanically by H.6/H.7 evidence nodes.

## M. Compatibility/version impact

| Contract | Classification |
|---|---|
| FullStateDigestV3 / EnvironmentCheckpointV3 / ReplayV3 | UNCHANGED |
| Decision V2 req/resp; InformationStateV2; ObservedEventV2; PlayerStepV2; EpisodeStatus; ObservationEnvelopeV1 | UNCHANGED shapes — MECHANICAL COVERAGE ONLY (new fixtures); Python V2-event validation TIGHTENED toward Rust (experimental surface, Rust-converging, negatively evidenced) |
| `information-state-digest-input.v2` | UNCHANGED (entry kept; modeled as pinned `PYTHON_MECHANICAL_ONLY` registry exception; standalone-promotion deferred) |
| Python package API | NEW EXPERIMENTAL NON-PUBLIC SURFACE (`_m2_adapter` incl. shape-only submission encoder; public `PlayerClient` protocol and codec APIs untouched) |
| Adapter protocol/commands | NEW EXPERIMENTAL NON-PUBLIC SURFACE (base64 payload confinement is envelope-local); never frozen; OD-009 OPEN |
| Shared fixture corpora | MECHANICAL COVERAGE ONLY (additive; historical baseline untouched) |

No version cut justified; no V3 reinterpretation; no new replay/checkpoint semantics; zero production-Rust API changes.

## N. Information-safety analysis

Privileged channels and why closed: **seeds/cursors** — trusted reset input only, never returned; seed-pair test proves invisibility. **Trusted IDs** — absent from player DTOs (ADR-0039/0040, M2.G); adapter forwards bytes, never re-serializes state. **Token existence/numbering** — OS-entropy tokens, never sequential, never echoed (request-id correlates); uniform lookup errors. **Envelope fields/length** — base64 of exactly the endpoint bytes; length = Rust's own output; seed-pair + byte-equality oracles catch secret-dependence. **stderr/debug** — separate stream, never consumed. **Command existence** — uniform `unknown_command`; trusted key scoped to the child environment. **Authoritative counters/replay/checkpoints** — no player op returns them; the tool cannot invoke checkpoint/restore/fork/export/execute_* (privilege scan) and holds replay types only as inert configuration identities. **Cross-perspective** — one token per client; wrong actor → uniform code; G.3 indistinguishability stands at the Rust layer, adapter adds no distinguishing channel.

## O. Determinism/replay/checkpoint analysis

Adapter state = {token registry, generation epoch} — **non-authoritative routing state**, never entering replay/checkpoint/trajectory/schema identity. Request ids and tokens correlate transport only and are EXCLUDED from all determinism comparisons. Semantic responses enter once via `submit_response_bytes` → the real endpoint; no continuation state outside the backend. Identical reset inputs + identical explicit answers reproduce byte-identical public payload sequences across processes (twins 1/4, restart 9). Post-panic termination guarantees no half-updated routing state reuse. No completeness claim depends on Python or adapter memory; recorder/checkpoint semantics untouched.

## P. Verification matrix

Nothing executed during planning. Every future implementation report marks unexecuted rows `NOT_RUN`/`BLOCKED`.

| Command | Owner | Status now |
|---|---|---|
| `cargo +1.85.1 fmt --all -- --check` | slice gates / PR Fast | NOT_RUN |
| `cargo +1.85.1 check --workspace --all-targets --all-features --locked` | idem | NOT_RUN |
| `cargo +1.85.1 clippy --workspace --all-targets --all-features --locked -- -D warnings` | idem | NOT_RUN |
| `cargo +1.85.1 test --workspace --all-features --locked` | idem | NOT_RUN |
| `python scripts/run_checks.py fast` | iterative during H.1–H.6 | NOT_RUN |
| `python scripts/run_checks.py integration` (`just check`) | before review-ready claims | NOT_RUN |
| `python scripts/run_m2_h_gates.py --development` | local iteration (never authoritative) | NOT_RUN |
| `python scripts/run_m2_h_gates.py --expect-commit <HEAD>` | authoritative local exact-head | NOT_RUN |
| Hosted PR Fast: checkout head.sha → assert == HEAD → clean asserts → B→G runs + **H runner** | hosted exact-head evidence | NOT_RUN |
| `run_checks.py certification` / full M1 matrix rerun | **M2.Final per ACCEPTANCE_GATES/ROADMAP — not H-owned** | NOT_RUN (deferred) |

## Q. Gate runner design

Fresh `run_m2_h_gates.py` on the G skeleton (import-time exact-set manifest validation + cross-gate uniqueness; REQUIRED_COVERAGE tag pinning; `exact_rust_pass`/`exact_python_pass` double-verbose single-node runs; toolchain snapshot; before/after fingerprint + porcelain; `dist/m2-h-verification` with ownership marker, never `dist/verification`). **Deliberate deviations (documented fixes):** per-node log indices, FAIL-dominant aggregation, per-node timeout → BLOCKED, structured startup errors, self-tests covering THIS runner. R4-specific content: REQUIRED_PLAYER_SURFACE signature extraction (completeness authority); per-contract SchemaContractDigest identities (semantic field-drift authority — every validation-relevant keyword through the closed `$ref` closure, non-semantic annotations excluded; computed from the checked-in schemas at runtime, pinned in the manifest, fail-closed on anomaly); Rust constructive producer nodes per gate-owned DTO (compile-barrier field-drift authority); derived transitive variant closures incl. optional-field presence/absence requirements; registry RELATION node per §G.6 (never bare equality); registry/schema/signature extraction anomalies fail closed. `--expect-commit` in authoritative mode; PR Fast supplies the exact-head assertion via the established +6-line suffix convention.

## R. Self-review

### R2 external findings → R3 resolutions

| Severity | Finding | Resolution in R3 |
|---|---|---|
| BLOCKER | Registry drift check (three-way equality) contradicts the deliberately kept digest-input asymmetry — permanent FAIL on the correct tree | Resolved: explicit set relation `COMMON_NAMED_CONTRACTS` + pinned `PYTHON_MECHANICAL_ONLY = {information-state-digest-input.v2}`; schema mapping pinned to COMMON; exception changes fail the gate until re-reviewed (§G.6, §Q) |
| BLOCKER | Dependency allowlist unimplementable: `SyntheticM1EnvironmentConfig` embeds `mtgml-replay` identity types; transitive forbidden-BFS can never pass (environment itself links all four) | Resolved: direct-allowlist ≠ transitive closure; `mtgml-replay` admitted for configuration identity types ONLY; forbidden direct set reduced to state/rules/persistence/conformance; operation-level privilege scan forbids replay/checkpoint FUNCTIONS in tool source (§D, §L, §J H.2/H.6) |
| BLOCKER | `episode_closed` unreachable: the synthetic kernel always emits `Running` on completion; forcing it would require a status-injection backchannel | Resolved: removed from the Gate-B runtime matrix; wire shapes stay Gate-A fixtures; semantics stay M2.D regression; one-row extension if the kernel ever terminates episodes natively (§H.5, §I) |
| MAJOR | `selftest_roundtrip` impossible: `decode_named` is private and production wire stays unchanged | Resolved: subcommand dropped entirely; cross-language authority = constructive encoding against the SAME checked-in fixtures on both sides (§G.3); tool shrinks; zero production-API changes |
| MAJOR | Twin token-vs-direct paths share the adapter handler → co-drift false-PASS possible | Resolved: mandatory below-JSONL handler-transparency tests for EVERY player operation (endpoint bytes == handler bytes, incl. counting-decorator zero-submit proof through the handler) (§H preamble, §J H.2) |
| MAJOR | JSON-string carriers cannot represent arbitrary invalid bytes; raw-byte layer-A testing incomplete | Resolved: envelope payloads are base64 (`*_wire_b64`), confined to the temporary shell; exact `&[u8]` reaches the Rust decoder; new byte-level corruption classes (invalid UTF-8, NUL, truncated multibyte, garbage) (§D, §H.6-item) |
| MAJOR | Surface closure missed DTO FIELD drift (e.g., new optional field tolerated by legacy fixtures) | Resolved: per-contract schema-shape structural digest (required/optional props, discriminators, `$ref` closure) pinned in the manifest; field change ⇒ FAIL ⇒ coherent manifest+fixture update; optional fields require presence-AND-absence coverage (§G.1, §Q) |
| MINOR | Trusted key must live only in the child environment | Resolved: child-scoped `env={…}` only; guard test proves parent `os.environ` untouched; `AdapterPlayerClient` gets a restricted transport, never a generic command handle (§D, §E, §J H.3) |
| MINOR | Determinism comparisons must exclude transport metadata | Resolved: global comparison-hygiene rule — payload bytes only; request ids/tokens/frame structure excluded everywhere (§H preamble, §O) |

### R3 external findings → R4 resolutions

| Severity | Finding | Resolution in R4 |
|---|---|---|
| MAJOR | `unavailable_decision` misclassified as 0 semantic submits — the real boundary calls `endpoint.submit()` once after layer-A decode succeeds; only the kernel transition is zero | Resolved: two-counter semantics (`endpoint_submit_calls` / `kernel_transition_attempts`) defined and applied throughout §H/§I — malformed 0/0 · typed rejections & wrong-actor 1/0 · accepted 1/1; an assertion of "0 submits" for `unavailable_decision` would itself bypass the real path and is flagged as a defect |
| MAJOR | Schema digest alone cannot catch Rust/Python DTO field drift when the schema itself is forgotten to be updated (signature unchanged, variants unchanged, old fixtures still pass) | Resolved: Rust constructive producer tests with explicit struct literals per gate-owned DTO added as a compile-barrier authority; four independent drift authorities now: Rust constructive + Python constructive + SchemaContractDigest + shared fixture bytes (§G.1, §Q) |
| MINOR | `invalid_cardinality` missing from reachable runtime classes | Resolved: added via below-minimum `SelectMany`; runtime matrix now EXACTLY the eight reachable classes, with `episode_closed` confined to Gate A fixtures + M2.D regression (§H.5, §I) |
| MINOR | Accepted-submit handler transparency must not consume an accepted state twice | Resolved: single-shot spy comparison (step returned by exactly one handler-driven submit vs bytes emitted by that same call) or twin fresh backends; same-instance transparency reserved for reads and rejected submits (§H preamble, §J H.2) |
| MINOR | "Schema-shape digest" naming invited a shape-only misreading (bounds/patterns assumed excluded) | Resolved: renamed SchemaContractDigest with explicit semantic coverage definition — every validation-relevant keyword through the closed `$ref` closure; only non-semantic annotations excluded (§G.1, §Q) |

### R4 external findings → R4.1 resolutions

| Severity | Finding | Resolution in R4.1 |
|---|---|---|
| MAJOR | `kernel_transition_attempts` claimed as a dynamic counter, but no hook can observe the internal transition call from outside `submit_player_response`; state-unchanged does not prove the kernel was never called; building such a counter would mean an unnecessary production instrumentation seam | Resolved: counter REMOVED; no production kernel hook exists or will be built; M2.H dynamically asserts only `endpoint_submit_calls` via the counting decorator around the REAL endpoint (0 for wire/envelope failures; exactly 1 for every typed submission incl. `unavailable_decision`); typed-rejection proofs = real-endpoint-once + player-visible nonmutation + cross-twin parity; full authoritative/replay nonmutation and pre-kernel rejection ownership remain accepted M2.D/M2.G evidence (§H item 5, §I) |
| MAJOR | Panic on a valid PLAYER command must not create a new player-visible `internal_error` class — the normative contract allows only `malformed_response` / closed submission codes / `service_unavailable` | Resolved: player-command panics map best-effort to the frozen layer-C `service_unavailable` then terminate immediately; unsafe emission ⇒ termination only + client-side transport closure; no trusted detail; `internal_error` reserved for trusted-only orchestration failures, unobservable through `AdapterPlayerClient` (§D failure policy, §I) |
| MINOR | Widening `submit` to `DecisionResponseV2 \| Mapping[str, object]` was unnecessary | Resolved: client signature stays protocol-exact typed; semantically-invalid values are built directly as dataclasses (only `validate()`/`to_wire()` judge); malformed-byte tests use the package-private `RestrictedPlayerTransport._submit_wire_bytes(token, bytes)` seam — one player operation, zero trusted commands (§E, §J H.3/H.6) |

### Residual risks / honest uncertainties in R4.1

| Severity | Category | Finding |
|---|---|---|
| MAJOR (accepted tension) | Information safety | Possession-based trusted/player separation in a shared process; externally ACCEPTED with the child-env precision; raw-stdin holder = orchestrator by definition. |
| MINOR | False-evidence fragility | Signature/registry extraction remains regex-based over small source regions; mitigated fail-closed (anomaly ⇒ FAIL, self-test pinned) and now COMPLEMENTED by SchemaContractDigest plus Rust constructive producer tests, whose compile failure on DTO field change is itself a hard, parser-free drift signal. |
| MINOR | Consciously accepted coverage boundary | `episode_closed` has no Gate-B runtime exercise (unreachable); wire + M2.D-regression ownership documented; revisit automatically if kernel gains native terminal episodes. |
| MINOR | Doc/spec gap (pre-existing) | Capability matrix lacks `reset`/`close` rows; ADR-0020 silent on invalidation; adapter policy documented here as non-normative; future docs follow-up, not filled normatively in M2.H. |
| MINOR | Maintainer ergonomics | Third copy of synthetic-config literals (deck-naming drift pre-exists between conformance/environment copies); parity oracles catch divergence. |
| NIT | Ergonomics | Dual mypy config pre-exists; H uses the justfile-invoked one only. |
| BLOCKER | — | None open in R3. |

Adversarial questions (§33, R3 status): 1 no (uniform errors) · 2 no (API + child-scoped key possession) · 3 no (epoch invalidation, uniform code) · 4 no (restricted transport; no generic command surface) · 5 no (`submit_response_bytes` seam, seam-proven through the handler) · 6 no (mapping payloads forwarded byte-verbatim; divergence test proves Rust judges) · 7 no (tests choose only offered public ids/values) · 8 no (continuations live in the backend) · 9 no (no RNG in Python/tool; token entropy non-semantic) · 10 no (base64 = exact endpoint bytes; seed-pair + transparency proofs) · 11 yes (envelope only on stdout) · 12 no (stderr separated) · 13 no (`direct_call` bound-handle trusted mode; transparency tests de-fang co-drift) · 14 no (unpublished, experimental, OD-009 intact) · 15 no (surface-signature + structural-digest + variant closure + registry relation + self-tests) · 16 no (twins assert real effects; transparency proofs anchor handler bytes to endpoint bytes) · 17 no (steps always adapter-response bytes vs trusted-direct twin) · 18 no (routing state non-authoritative; panic ⇒ terminate) · 19 no (restart node enforces payload-byte equality, metadata excluded) · 20 no (helper inventory + symbol scans + no-generic-send check).

## S. Final recommendation

**IMPLEMENTATION GO (R4.1) — external review verdict recorded.**

The reviewer's final assessment: architecture APPROVED (R4 verdict: BLOCKER 0 / MAJOR 2 / MINOR 1), with formal **IMPLEMENTATION GO effective once exactly these three amendments were incorporated — they now are**:

1. **No dynamic kernel counter.** `kernel_transition_attempts` is removed as an executable claim; there is deliberately NO production kernel instrumentation seam and NO new test hook. M2.H dynamically proves ONLY `endpoint_submit_calls` — malformed/envelope failures = 0; every typed submission including `unavailable_decision` = exactly 1 (counting decorator around the real endpoint, the established M2.G seam pattern). Typed rejections are proven by real-endpoint-once + player-visible nonmutation + cross-twin parity; complete authoritative/replay nonmutation and pre-kernel rejection ownership remain the already-accepted M2.D/M2.G evidence.
2. **No fourth player error class.** A panic while servicing a valid PLAYER command maps best-effort to the frozen layer-C `service_unavailable` surface and terminates the process immediately; if emission is unsafe, termination only and Python observes transport closure. The player boundary remains EXACTLY `{malformed_response, closed submission codes, service_unavailable}`; `internal_error` exists only for trusted setup/orchestration failures and is unobservable through `AdapterPlayerClient`.
3. **Protocol-exact typed submit.** `AdapterPlayerClient.submit(response: DecisionResponseV2) -> PlayerStepV2` matches the `PlayerClient` protocol precisely; semantically-invalid responses ride as directly-constructed dataclass instances through the shape-only encoder; malformed/raw-byte testing uses the package-private `RestrictedPlayerTransport._submit_wire_bytes(token, bytes)` seam (one player operation, zero trusted commands).

Review trajectory: R1 BLOCKER 3/MAJOR 3/MINOR 2 → R2 3/4/2 → R3 0/2/3 → R4 0/2/1 → R4.1 amendments incorporated.

Implementation begins from current `master` (`e250f6e06a1c1af65a53cd7b1e986fbcf1958644`) on the suggested branch `chris/m2-h-python-adapter`, executing slices H.1–H.7 in order (H.2/H.3 parallelizable after H.1; H.7 last), with every gate in §P starting NOT_RUN and claiming PASS only on executed exact-head evidence.
