#!/usr/bin/env python3
"""In-process mutation sweep for the pages copy checker.

The checker is Python, so this is the reproducible stand-in for a cargo-mutants
run. It splices scripts/ci/test_workflow_policy.py and reruns the pages tests
in this process. test_shell_state_cap_bounds_conditional_directories is not
part of the suite: that test reloads the file from disk and cannot see an
in-process splice. test_shell_state_cap_boundary calls _shell_states directly.

Scope is every statement and constant from BENCHMARK_ALERT_PUBLISH_DIR through
assert_docs_no_archive_benchmark_dashboard_copy, plus _call_checker.

Operators: delete a statement, return None, swap a comparison, swap and/or,
drop not, flip a boolean, replace an int with 0, 1, or n+1, replace a code
string with Z or empty, swap break/continue, drop one element of a constant
set, list, or tuple. A splice that does not parse is unviable and is not run.
Each mutant times out after 20 seconds. Four worker processes.

Exit 0 prints the counts. A survivor is a mutant the suite still passes.
"""

from __future__ import annotations

import ast
import copy
import subprocess
import sys
import textwrap
from concurrent.futures import ProcessPoolExecutor, as_completed
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
POLICY = ROOT / "scripts" / "ci" / "test_workflow_policy.py"
TIMEOUT_S = 20
WORKERS = 4
TESTS = (
    "test_pages_publish_path_helpers",
    "test_shell_state_cap_boundary",
    "test_identical_conditional_directories_stay_deduped",
    "test_literal_expression_scan_terminates",
    "test_scan_loops_terminate",
    "test_parameter_expansion_and_option_prefixes",
    "test_call_checker_reports_a_crash",
    "test_benchmark_alert_gh_pages_publish_paths",
    "test_docs_no_archive_benchmark_dashboard_copy",
)

_CMP = {
    ast.Eq: ast.NotEq,
    ast.NotEq: ast.Eq,
    ast.Lt: ast.Gt,
    ast.Gt: ast.Lt,
    ast.LtE: ast.GtE,
    ast.GtE: ast.LtE,
    ast.Is: ast.IsNot,
    ast.IsNot: ast.Is,
    ast.In: ast.NotIn,
    ast.NotIn: ast.In,
}

_RUNNER = textwrap.dedent(
    """\
    import io
    import sys
    import types
    import unittest

    source = sys.stdin.read()
    module = types.ModuleType("policy_mutant")
    module.__file__ = sys.argv[1]
    module.__dict__["__file__"] = sys.argv[1]
    try:
        exec(compile(source, sys.argv[1], "exec"), module.__dict__)
    except Exception:
        sys.exit(3)
    suite = unittest.TestSuite()
    for name in sys.argv[2].split(","):
        suite.addTest(module.WorkflowPolicyTest(name))
    stream = io.StringIO()
    result = unittest.TextTestRunner(stream=stream, verbosity=0, failfast=True).run(suite)
    sys.exit(0 if result.wasSuccessful() else 1)
    """
)


def _ranges(tree: ast.AST) -> tuple[tuple[int, int], ...]:
    start = end = call_start = call_end = None
    for node in tree.body:
        if isinstance(node, ast.Assign):
            names = [target.id for target in node.targets if isinstance(target, ast.Name)]
            if "BENCHMARK_ALERT_PUBLISH_DIR" in names:
                start = node.lineno
        if isinstance(node, ast.FunctionDef) and node.name == "assert_docs_no_archive_benchmark_dashboard_copy":
            end = node.end_lineno
        if isinstance(node, ast.FunctionDef) and node.name == "_call_checker":
            call_start = node.lineno
            call_end = node.end_lineno
    if None in (start, end, call_start, call_end):
        raise RuntimeError("pages checker range is missing")
    return ((start, end), (call_start, call_end))


def _in_range(lineno: int | None, ranges: tuple[tuple[int, int], ...]) -> bool:
    if lineno is None:
        return False
    return any(start <= lineno <= end for start, end in ranges)


def _line_starts(source: str) -> list[int]:
    starts = [0]
    for line in source.splitlines(keepends=True):
        starts.append(starts[-1] + len(line))
    return starts


def _span(starts: list[int], node: ast.AST) -> tuple[int, int] | None:
    if not hasattr(node, "lineno") or not hasattr(node, "end_lineno"):
        return None
    end_col = getattr(node, "end_col_offset", None)
    if end_col is None or node.end_lineno is None:
        return None
    return starts[node.lineno - 1] + node.col_offset, starts[node.end_lineno - 1] + end_col


def _parents(tree: ast.AST) -> dict[ast.AST, ast.AST]:
    found: dict[ast.AST, ast.AST] = {}
    for node in ast.walk(tree):
        for child in ast.iter_child_nodes(node):
            found[child] = node
    return found


def _docstring(node: ast.AST, parent: ast.AST | None) -> bool:
    if parent is None or not isinstance(parent, (ast.FunctionDef, ast.AsyncFunctionDef, ast.ClassDef, ast.Module)):
        return False
    body = getattr(parent, "body", [])
    if not body or body[0] is not node:
        return False
    return (
        isinstance(node, ast.Expr)
        and isinstance(node.value, ast.Constant)
        and isinstance(node.value.value, str)
    )


def _splice(source: str, start: int, end: int, replacement: str) -> str | None:
    mutant = source[:start] + replacement + source[end:]
    if mutant == source:
        return None
    try:
        ast.parse(mutant)
    except SyntaxError:
        return None
    return mutant


def _replace_node(source: str, starts: list[int], node: ast.AST, replacement: ast.AST) -> str | None:
    span = _span(starts, node)
    if span is None:
        return None
    return _splice(source, span[0], span[1], ast.unparse(replacement))


def generate(source: str) -> tuple[list[tuple[int, str, str]], int, tuple[tuple[int, int], ...]]:
    """Return parseable mutants as (line, kind, source), the unviable count, and the line ranges."""
    tree = ast.parse(source)
    ranges = _ranges(tree)
    starts = _line_starts(source)
    parent = _parents(tree)
    mutants: list[tuple[int, str, str]] = []
    unviable = 0
    seen: set[str] = set()

    def add(line: int, kind: str, mutant: str | None, *, failed_parse: bool = False) -> None:
        nonlocal unviable
        if failed_parse:
            unviable += 1
            return
        if mutant is None or mutant in seen:
            return
        seen.add(mutant)
        mutants.append((line, kind, mutant))

    for node in ast.walk(tree):
        if not _in_range(getattr(node, "lineno", None), ranges):
            continue
        if _docstring(node, parent.get(node)):
            continue
        if isinstance(node, ast.stmt) and not isinstance(node, (ast.FunctionDef, ast.AsyncFunctionDef, ast.ClassDef)):
            span = _span(starts, node)
            if span is not None:
                deleted = source[: span[0]] + source[span[1] :]
                try:
                    ast.parse(deleted)
                except SyntaxError:
                    add(node.lineno, "delete-statement", None, failed_parse=True)
                else:
                    add(node.lineno, "delete-statement", deleted if deleted != source else None)
        if isinstance(node, ast.Return) and node.value is not None:
            if not (isinstance(node.value, ast.Constant) and node.value.value is None):
                changed = copy.deepcopy(node)
                changed.value = ast.Constant(value=None)
                add(node.lineno, "return-none", _replace_node(source, starts, node, changed))
        if isinstance(node, ast.Compare) and len(node.ops) == 1:
            swapped = _CMP.get(type(node.ops[0]))
            if swapped is not None:
                changed = copy.deepcopy(node)
                changed.ops = [swapped()]
                add(node.lineno, "compare-swap", _replace_node(source, starts, node, changed))
        if isinstance(node, ast.BoolOp):
            changed = copy.deepcopy(node)
            changed.op = ast.Or() if isinstance(node.op, ast.And) else ast.And()
            add(node.lineno, "bool-swap", _replace_node(source, starts, node, changed))
        if isinstance(node, ast.UnaryOp) and isinstance(node.op, ast.Not):
            add(node.lineno, "drop-not", _replace_node(source, starts, node, node.operand))
        if isinstance(node, ast.Constant):
            value = node.value
            if isinstance(value, bool):
                changed = ast.Constant(value=not value)
                add(node.lineno, "bool-flip", _replace_node(source, starts, node, changed))
            elif isinstance(value, int) and not isinstance(value, bool):
                for target in (0, 1, value + 1):
                    if target == value:
                        continue
                    add(
                        node.lineno,
                        f"int-{target}",
                        _replace_node(source, starts, node, ast.Constant(value=target)),
                    )
            elif isinstance(value, str) and parent.get(node) is not None and not _docstring(parent[node], parent.get(parent[node])):
                for target in ("Z", ""):
                    if target == value:
                        continue
                    add(
                        node.lineno,
                        "string",
                        _replace_node(source, starts, node, ast.Constant(value=target)),
                    )
        if isinstance(node, (ast.Break, ast.Continue)):
            changed: ast.AST = ast.Continue() if isinstance(node, ast.Break) else ast.Break()
            add(node.lineno, "break-continue", _replace_node(source, starts, node, changed))
        if isinstance(node, (ast.Set, ast.List, ast.Tuple)) and len(node.elts) >= 2:
            if all(isinstance(elt, ast.Constant) for elt in node.elts):
                for index in range(len(node.elts)):
                    changed = copy.deepcopy(node)
                    del changed.elts[index]
                    add(node.lineno, f"drop-elt-{index}", _replace_node(source, starts, node, changed))
    return mutants, unviable, ranges


def _run_one(mutant: str) -> str:
    try:
        completed = subprocess.run(
            [sys.executable, "-c", _RUNNER, str(POLICY), ",".join(TESTS)],
            input=mutant,
            text=True,
            capture_output=True,
            timeout=TIMEOUT_S,
            check=False,
        )
    except subprocess.TimeoutExpired:
        return "timeout"
    if completed.returncode == 0:
        return "survived"
    if completed.returncode == 3:
        return "crash"
    return "killed"


def main() -> int:
    source = POLICY.read_text(encoding="utf-8")
    mutants, unviable, ranges = generate(source)
    print(
        f"scope: {POLICY} lines {ranges[0][0]}-{ranges[0][1]} and {ranges[1][0]}-{ranges[1][1]}",
        flush=True,
    )
    print(f"generated: {len(mutants)} viable, {unviable} unviable", flush=True)
    counts = {"killed": 0, "survived": 0, "timeout": 0, "crash": 0}
    survivors: list[tuple[int, str]] = []
    with ProcessPoolExecutor(max_workers=WORKERS) as pool:
        future_map = {pool.submit(_run_one, mutant): (line, kind) for line, kind, mutant in mutants}
        done = 0
        for future in as_completed(future_map):
            status = future.result()
            counts[status] += 1
            if status == "survived":
                survivors.append(future_map[future])
            done += 1
            if done % 200 == 0:
                print(f"progress {done}/{len(mutants)} {counts}", flush=True)
    print(
        f"killed={counts['killed']} survived={counts['survived']} "
        f"unviable={unviable} timeout={counts['timeout']} crash={counts['crash']}",
        flush=True,
    )
    for line, kind in sorted(survivors):
        print(f"survivor {line} {kind}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
