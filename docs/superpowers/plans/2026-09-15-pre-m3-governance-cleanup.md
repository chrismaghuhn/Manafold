# Final Pre-M3 Governance Cleanup Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Reconcile current pre-M3 status, close the OD-004 and OD-019 entry decisions, classify every OD-001 through OD-021 against M3 entry, and publish one reviewable governance PR without authorizing M3.

**Status:** active execution plan

**Architecture:** Keep this change documentation-only. Bind the first M3 rules authority to one exact official Wizards artifact by a small provenance/digest record and accepted ADR 0051; bind the initial M3 format boundary to `FormatState::None` with accepted ADR 0052, while deferring generic hooks until concrete semantic evidence requires a typed seam.

**Tech Stack:** Markdown, JSON documentation register if required by existing policy, PowerShell, Git, GitHub CLI, repository documentation/verification scripts, and the existing `just` profiles.

---

### Task 1: Verify base, current project state, and decision evidence

**Files:**
- Read: `AGENTS.md`, `README.md`, `docs/NORMATIVE_HIERARCHY.md`, `docs/ROADMAP.md`, `docs/OPEN_DECISIONS.md`
- Read: `docs/rules/AUTHORITY_POLICY.md`, `docs/FORMAT_MODULES.md`, rule/capability/certification docs, ADR 0022, ADR 0041, `docs/contracts/ACCEPTANCE_GATES.md`
- Read: current freeze evidence and ADR index
- Read: GitHub issues #178, #163, #129, #162 and current PR #177/#179 status

- [x] Fetch `origin/master`, verify it equals `b4029d287a5412678cc5088ddc6cb6af063c8e7a`, and record the intervening PR #179 merge as the current base.
- [x] Confirm #105 and #164 are closed, #129 and #162 are complete/closed, #163 remains planning-only, and #178 remains the separate unauthorized M3 entry tracker.
- [x] Search current-status prose for stale active references to #105, the pre-M3 adversarial audit, foundation reconciliation, and #162; preserve historical records.
- [x] Confirm ADR 0051 and ADR 0052 are the next unreserved numbers after ADR 0050.

### Task 2: Establish the official Comprehensive Rules snapshot identity

**Files:**
- Modify: `docs/rules/COMPREHENSIVE_RULES_ARTIFACT_RESEARCH_2026-09-15.md`
- Read: official Wizards source and exact downloaded bytes

- [x] Use the official Wizards source only as final authority and determine the current applicable Comprehensive Rules artifact as of 2026-09-15.
- [x] Record publisher, official URL, file/document name, effective or publication date/version, retrieval timestamp, media type, exact byte length, and SHA-256 of the exact official bytes.
- [x] Explain why the selected artifact is authoritative for Manafold's first M3 rules case and keep Oracle, rulings, format policy, and banlist identities separate.
- [x] Do not vendor the copyrighted rules document; preserve only the locator and exact identity metadata.

### Task 3: Resolve OD-004 in ADR 0051

**Files:**
- Create: `docs/adr/0051-comprehensive-rules-snapshot-authority.md`
- Modify: `docs/adr/README.md`

- [x] Record the selected authority and stable project-facing snapshot identifier bound to the exact provenance record and digest.
- [x] Define conformance citation requirements: snapshot identity, CR section/rule numbers, and additional official ruling authority where needed.
- [x] Define update/migration behavior so old evidence remains bound to its old snapshot and new work explicitly selects a snapshot with reviewed impact analysis/revalidation.
- [x] State that the decision authorizes the first real M3 rules case but implements no rule, selects no S1, and certifies no capability.
- [x] Mark ADR 0051 accepted only when the decision is complete and the exact identity is present.

### Task 4: Resolve the initial M3 format boundary in ADR 0052

**Files:**
- Create: `docs/adr/0052-initial-m3-format-neutral-boundary.md`
- Modify: `docs/FORMAT_MODULES.md`
- Modify: `docs/adr/README.md`

- [x] Preserve accepted format ownership and authoritative `EngineState.format` ownership.
- [x] Resolve the initial M3 boundary as format-neutral with `FormatState::None`.
- [x] State that no generic/dynamic hook API is required or frozen for initial M3, while future concrete typed seams remain possible only after reviewed semantic evidence.
- [x] Preserve the restrictions against hidden mutable state, direct authoritative commits, silent choices, external side effects, alternate rules engines, and dynamic third-party registration.
- [x] Mark ADR 0052 accepted only when it clearly resolves the M3 deadline portion of OD-019.

### Task 5: Reconcile status prose and audit all OD rows

**Files:**
- Modify: `README.md`
- Modify: `docs/ROADMAP.md`
- Modify: `docs/OPEN_DECISIONS.md`
- Modify: `python/tests/test_current_status.py` (status guard only)

- [x] Update current status to foundation frozen/complete, #162 complete, governance cleanup current, M3 not started/not authorized, and M3 entry decision next.
- [x] Review OD-001 through OD-021 individually, preserving the existing compact register vocabulary and recording current status, deadline, safe default/current resolution, M3 relevance, and evidence without silently resolving unsupported choices.
- [x] Move OD-003's durable deadline to M4 benchmark/content/bundle planning or first exact deck benchmark freeze; keep fail-closed no-support/no-benchmark semantics.
- [x] Mark OD-004 and OD-019 resolved only after ADR 0051/0052 are complete.
- [x] Classify all remaining open/partial rows; explicitly identify downstream/non-M3 items and any real M3 blocker. Do not mark #178 entry approval or authorization.
- [x] Update `Last reviewed` to 2026-09-15.

### Task 6: Verify scope and repository gates

**Files:**
- Read-only verification of the complete diff and source tree

- [ ] Confirm no production Rust/Python, capability registry, Card IR, wire, schema, replay, RNG, digest, or authoritative state files changed.
- [ ] Run `python scripts/check_documentation.py`, `python scripts/verify_repository.py`, and `git diff --check`.
- [ ] Run `just check-fast` and `just check` when wrappers work; otherwise record wrapper status as `BLOCKED` and run exact native equivalents required by the repository.
- [ ] Inspect the final diff and re-run all status/search checks after the last edit.

### Task 7: Commit, publish, review, and report

**Files:**
- Git commits on `chris/pre-m3-governance-cleanup-20260915`

- [ ] Commit logically reviewable documentation changes without rewriting history or merging.
- [ ] Push the branch and open one PR titled `docs: close pre-M3 governance decisions` with an explicit non-authorization/non-semantics/non-S1/non-registry scope statement.
- [ ] Verify hosted PR Fast, PR Integration, `manafold-pr-gate`, and Windows Setup Smoke results; do not report pending checks as green.
- [ ] Add the concise candidate/awaiting-merge comment to #178 only after the PR exists; do not close #178 or authorize M3.
- [ ] Finish with the exact requested status block, including evidence, blockers, wrapper status, PR, branch, base, and final remote master.
