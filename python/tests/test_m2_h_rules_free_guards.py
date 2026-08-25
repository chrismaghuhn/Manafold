"""H.6 rules-free Python static authority guards (Issue #55).

Static, dependency-free source authority checks over ``python/src/mtgml``
and ``python/pyproject.toml``. These guards are the PYTHON-side mirror of
the Rust-side dependency/privilege nodes that ALREADY exist as in-crate
``#[cfg(test)]`` tests in ``tools/m2-semantic-adapter/src/evidence_tests.rs``
(the cargo-metadata allowlist guard including the workspace-wide
no-depends-on-tool check, plus the identifier-aware privilege-source scan
with its own self-test). They deliberately do NOT duplicate those gates:
this file reads repository text only — it never spawns a process, never
executes the adapter, and needs no binary, network, or third-party package.

Guard inventory (each fails LOUDLY with aggregated ``file:line``
diagnostics):

1. ZERO-RUNTIME-DEPS: ``[project] dependencies`` in
   ``python/pyproject.toml`` must be exactly ``[]``, and
   ``[project.optional-dependencies]`` must not introduce any runtime
   surface (when present, the value must be a table and every extra
   group in it must be empty).
   Current reality, documented: ``dependencies == []`` and no
   optional-dependencies table exists at all.
2. IMPORT CONFINEMENT: every absolute ``import`` / ``from ... import``
   statement under ``python/src/mtgml`` (recursive, including
   ``TYPE_CHECKING`` blocks) must resolve to the standard library or to
   ``mtgml`` itself. The ``random``, ``numpy``, ``torch``, ``ctypes``,
   ``cffi``, and ``multiprocessing`` top-level modules are FORBIDDEN
   anywhere. ``subprocess`` is permitted ONLY within the ``_m2_adapter``
   subtree (trusted orchestration plumbing by design) and must never
   appear elsewhere. Relative imports stay inside ``mtgml`` by
   construction.
3. NO HIDDEN CHOICE-MAKING: the forbidden choice/heuristic symbols
   (``legal_actions``, ``best_action``, ``auto_step``, ``auto_answer``,
   ``auto_complete_response``, ``repair_response``,
   ``resolve_candidates``, ``choose_action``, ``pick_action``,
   ``decide_action``) must have zero occurrences anywhere in package
   source. Decisions are supplied by callers, never made here. Verified
   empirically clean before this guard was frozen.
4. TRUSTED-VOCABULARY BOUNDARY on the player-facing surface (all of
   ``_m2_adapter/submission.py`` plus every line of
   ``_m2_adapter/client.py`` EXCEPT one class): the substrings
   ``root_seed``, ``trusted_key``, ``checkpoint``, ``fork``,
   ``export_replay``, ``ContinuationId``, ``GameObjectId``,
   ``rng_state``, and ``cursor`` must not appear. Boundary encoded
   here: ``process.py`` and ``protocol.py`` are exempt because they ARE
   the trusted orchestration plumbing by design; for the same reason
   the ``SyntheticEnvironmentClient`` class body inside ``client.py``
   (whose reset/bind/shutdown commands must carry the trusted key
   internally) is out of scan scope, while everything else in
   ``client.py`` — notably the entire ``AdapterPlayerClient`` class,
   its module helpers included — IS scanned. Trusted vocabulary
   reaching that player-facing surface fails this guard.
5. API INVENTORY CONSOLIDATION: re-asserts the exact public method sets
   of ``AdapterPlayerClient`` and ``SyntheticEnvironmentClient`` and
   the static ``PlayerClient`` protocol witness, reusing the pinned
   inventories from ``tests/m2_h/harness.py`` (single definition; see
   ``PLAYER_CLIENT_PUBLIC_METHODS`` / ``SYNTHETIC_PUBLIC_METHODS``
   there) instead of duplicating literals.

All scans decode sources as UTF-8 with BOM tolerance (``utf-8-sig``);
line endings (CRLF or LF) never affect matching or line accounting.
Every reported ``file:line`` coordinate is a true on-disk line number,
and decode/parse failures always name the offending file.
"""

from __future__ import annotations

import ast
import sys
import tomllib
import unittest
from collections.abc import Callable, Iterator
from pathlib import Path
from typing import Any, Final, cast

_PYTHON_DIR = Path(__file__).resolve().parents[1]
_SRC_ROOT = _PYTHON_DIR / "src"
sys.path.insert(0, str(_SRC_ROOT))

from m2_h import harness
from mtgml._m2_adapter import (
    AdapterPlayerClient,
    SyntheticEnvironmentClient,
)

PACKAGE_ROOT: Final = _SRC_ROOT / "mtgml"

FORBIDDEN_TOP_LEVEL_MODULES: Final[frozenset[str]] = frozenset(
    {"random", "numpy", "torch", "ctypes", "cffi", "multiprocessing"}
)
SUBPROCESS_MODULE: Final = "subprocess"
ADAPTER_SUBTREE_NAME: Final = "_m2_adapter"
PACKAGE_TOP_LEVEL: Final = "mtgml"

FORBIDDEN_CHOICE_SYMBOLS: Final[tuple[str, ...]] = (
    "legal_actions",
    "best_action",
    "auto_step",
    "auto_answer",
    "auto_complete_response",
    "repair_response",
    "resolve_candidates",
    "choose_action",
    "pick_action",
    "decide_action",
)

FORBIDDEN_TRUSTED_VOCABULARY: Final[tuple[str, ...]] = (
    "root_seed",
    "trusted_key",
    "checkpoint",
    "fork",
    "export_replay",
    "ContinuationId",
    "GameObjectId",
    "rng_state",
    "cursor",
)

TRUSTED_PLUMBING_FILES: Final[frozenset[str]] = frozenset({"process.py", "protocol.py"})
TRUSTED_PLUMBING_CLASS: Final = "SyntheticEnvironmentClient"
PROTOCOL_WITNESS_NAME: Final = "_protocol_witness"
PROTOCOL_CLASS_NAME: Final = "PlayerClient"


def _package_sources() -> list[Path]:
    return sorted(PACKAGE_ROOT.rglob("*.py"))


def _read_source(path: Path) -> str:
    """BOM-tolerant decode; universal newlines make CRLF/LF irrelevant."""
    try:
        return path.read_bytes().decode("utf-8-sig")
    except UnicodeDecodeError as decode_error:
        raise UnicodeDecodeError(
            decode_error.encoding,
            decode_error.object,
            decode_error.start,
            decode_error.end,
            f"{_relative(path)}: {decode_error.reason}",
        ) from decode_error


def _parse_source(path: Path, source: str) -> ast.Module:
    """Parse scanned source; any SyntaxError names the on-disk file."""
    try:
        return ast.parse(source)
    except SyntaxError as syntax_error:
        syntax_error.filename = _relative(path)
        raise


def _relative(path: Path) -> str:
    return path.relative_to(_PYTHON_DIR).as_posix()


def _line_of(source: str, offset: int) -> int:
    return source.count("\n", 0, offset) + 1


def _absolute_import_tops(tree: ast.Module) -> list[tuple[int, str]]:
    """(lineno, top-level module name) for every ABSOLUTE import site."""
    sites: list[tuple[int, str]] = []
    for node in ast.walk(tree):
        if isinstance(node, ast.Import):
            for alias in node.names:
                sites.append((node.lineno, alias.name.split(".")[0]))
        elif isinstance(node, ast.ImportFrom):
            if node.level > 0:
                continue  # relative imports resolve inside mtgml by construction
            if node.module is None:  # pragma: no cover - impossible when level == 0
                continue
            sites.append((node.lineno, node.module.split(".")[0]))
    return sites


def _occurrences(source: str, marker: str) -> Iterator[int]:
    """Every start offset of ``marker`` in ``source``."""
    offset = source.find(marker)
    while offset != -1:
        yield offset
        offset = source.find(marker, offset + 1)


def _numbered_lines(_path: Path, source: str) -> list[tuple[int, str]]:
    """(on-disk 1-based line number, text) for every line of ``source``."""
    return list(enumerate(source.split("\n"), start=1))


def _lines_outside_trusted_class(path: Path, source: str) -> list[tuple[int, str]]:
    """``_numbered_lines`` minus the trusted plumbing class body.

    Excluded lines are SKIPPED, never stripped-and-renumbered, so scan
    coordinates stay true on-disk line numbers. Line-at-a-time scanning
    is exact because no forbidden marker spans a line break.
    """
    tree = _parse_source(path, source)
    excluded_spans = [
        (node.lineno, node.end_lineno)
        for node in tree.body
        if isinstance(node, ast.ClassDef) and node.name == TRUSTED_PLUMBING_CLASS
    ]
    return [
        (number, line)
        for number, line in _numbered_lines(path, source)
        if not any(start <= number <= end for start, end in excluded_spans)
    ]


PLAYER_SURFACE_TRANSFORMS: Final[dict[str, Callable[[Path, str], list[tuple[int, str]]]]] = {
    "client.py": _lines_outside_trusted_class,
    "submission.py": _numbered_lines,
}


def _type_checking_names(tree: ast.Module) -> set[str]:
    return {
        node.name
        for conditional in tree.body
        if isinstance(conditional, ast.If)
        and isinstance(conditional.test, ast.Name)
        and conditional.test.id == "TYPE_CHECKING"
        for node in conditional.body
        if isinstance(node, ast.FunctionDef | ast.AsyncFunctionDef)
    }


class RulesFreeStaticGuardsTests(unittest.TestCase):
    """The five H.6 authority guards. Purely static; never skipped."""

    def test_pyproject_declares_zero_runtime_dependencies(self) -> None:
        document = tomllib.loads(_read_source(_PYTHON_DIR / "pyproject.toml"))
        project = document.get("project")
        self.assertIsInstance(project, dict, "python/pyproject.toml lacks the [project] table")
        assert isinstance(project, dict)
        self.assertEqual(
            project.get("dependencies"),
            [],
            "[project] dependencies must remain exactly []; the rules-free "
            "Python contracts ship with zero runtime packages",
        )
        extras = project.get("optional-dependencies")
        offending: dict[str, Any] = {}
        if extras is not None:
            self.assertIsInstance(
                extras,
                dict,
                "[project] optional-dependencies must be a table of extra "
                "groups; anything else is malformed and fails closed",
            )
            assert isinstance(extras, dict)
            offending = {group: entries for group, entries in extras.items() if entries}
        self.assertEqual(
            offending,
            {},
            f"[project] optional-dependencies would introduce runtime "
            f"surface: {offending}; keep extras empty or remove the table "
            "(documented current reality: the table does not exist)",
        )

    def test_import_scan_confines_imports_to_stdlib_and_mtgml(self) -> None:
        violations: list[str] = []
        sources = _package_sources()
        self.assertTrue(sources, "import scan found no files under python/src/mtgml")
        for path in sources:
            source = _read_source(path)
            relative = _relative(path)
            in_adapter_subtree = ADAPTER_SUBTREE_NAME in path.parts
            for lineno, top in _absolute_import_tops(_parse_source(path, source)):
                where = f"{relative}:{lineno}"
                if top in FORBIDDEN_TOP_LEVEL_MODULES:
                    violations.append(f"{where}: forbidden module {top!r}")
                elif top == SUBPROCESS_MODULE and not in_adapter_subtree:
                    violations.append(
                        f"{where}: {SUBPROCESS_MODULE!r} outside the "
                        f"{ADAPTER_SUBTREE_NAME}/ subtree"
                    )
                elif top not in sys.stdlib_module_names and top != PACKAGE_TOP_LEVEL:
                    violations.append(
                        f"{where}: import {top!r} resolves outside stdlib+{PACKAGE_TOP_LEVEL}"
                    )
        self.assertEqual(
            violations,
            [],
            "import-confinement violations:\n  " + "\n  ".join(violations),
        )

    def test_forbidden_choice_symbols_are_absent_from_source(self) -> None:
        hits: list[str] = []
        sources = _package_sources()
        self.assertTrue(sources, "symbol scan found no files under python/src/mtgml")
        for path in sources:
            source = _read_source(path)
            relative = _relative(path)
            for symbol in FORBIDDEN_CHOICE_SYMBOLS:
                for offset in _occurrences(source, symbol):
                    hits.append(f"{relative}:{_line_of(source, offset)}: {symbol!r}")
        self.assertEqual(
            hits,
            [],
            "forbidden choice-making symbols present:\n  " + "\n  ".join(hits),
        )

    def test_player_facing_surface_carries_no_trusted_vocabulary(self) -> None:
        hits: list[str] = []
        for name, transform in PLAYER_SURFACE_TRANSFORMS.items():
            path = PACKAGE_ROOT / ADAPTER_SUBTREE_NAME / name
            relative = _relative(path)
            for number, line in transform(path, _read_source(path)):
                for marker in FORBIDDEN_TRUSTED_VOCABULARY:
                    for _offset in _occurrences(line, marker):
                        hits.append(f"{relative}:{number}: {marker!r}")
        self.assertEqual(
            hits,
            [],
            "trusted vocabulary on the player-facing surface:\n  " + "\n  ".join(hits),
        )

    def test_public_api_inventory_matches_the_pinned_witnesses(self) -> None:
        unused_transport = cast(Any, object())  # clients only store the reference
        player = AdapterPlayerClient(unused_transport, "inventory-witness")
        environment = SyntheticEnvironmentClient(unused_transport)
        self.assertEqual(
            harness.public_method_names(player),
            set(harness.PLAYER_CLIENT_PUBLIC_METHODS),
            "AdapterPlayerClient's public method set drifted from the pinned inventory",
        )
        self.assertEqual(
            harness.public_method_names(environment),
            set(harness.SYNTHETIC_PUBLIC_METHODS),
            "SyntheticEnvironmentClient's public method set drifted from the pinned inventory",
        )

        protocol_path = PACKAGE_ROOT / "player_client.py"
        protocol_tree = _parse_source(protocol_path, _read_source(protocol_path))
        protocol_classes = [
            node
            for node in protocol_tree.body
            if isinstance(node, ast.ClassDef) and node.name == PROTOCOL_CLASS_NAME
        ]
        self.assertEqual(len(protocol_classes), 1, "expected exactly one PlayerClient")
        protocol_methods = {
            node.name
            for node in protocol_classes[0].body
            if isinstance(node, ast.FunctionDef | ast.AsyncFunctionDef)
            and not node.name.startswith("_")
        }
        self.assertEqual(
            protocol_methods,
            set(harness.PLAYER_CLIENT_PUBLIC_METHODS),
            "the PlayerClient protocol drifted from the pinned client inventory",
        )

        client_path = PACKAGE_ROOT / ADAPTER_SUBTREE_NAME / "client.py"
        client_source = _read_source(client_path)
        client_tree = _parse_source(client_path, client_source)
        self.assertIn(
            PROTOCOL_WITNESS_NAME,
            _type_checking_names(client_tree),
            "the TYPE_CHECKING-only PlayerClient witness disappeared from client.py",
        )
        witnesses = [
            node
            for node in ast.walk(client_tree)
            if isinstance(node, ast.FunctionDef | ast.AsyncFunctionDef)
            and node.name == PROTOCOL_WITNESS_NAME
        ]
        self.assertEqual(len(witnesses), 1, "expected exactly one protocol witness")
        returns = witnesses[0].returns
        self.assertIsNotNone(returns, "the protocol witness lost its return annotation")
        assert returns is not None
        self.assertEqual(
            ast.unparse(returns),
            PROTOCOL_CLASS_NAME,
            "the protocol witness no longer proves AdapterPlayerClient satisfies PlayerClient",
        )


if __name__ == "__main__":
    unittest.main()
