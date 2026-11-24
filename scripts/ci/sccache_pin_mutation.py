#!/usr/bin/env python3
"""AST mutation pass for the sccache pin checker.

The checker is the sccache pin block in scripts/ci/test_workflow_policy.py,
from SCCACHE_ACTION_OWNER through repo_sccache_action_sites. Operators:

- flip compares (==/!=, in/not in, is/is not, < />) and boundary compares (< /<=, >/>=)
- swap and/or, and drop each operand
- invert if tests and if-expressions
- swap continue and break
- flip True and False
- replace 0.19.0 inside a string constant with 0.20.0
- replace each return value with [] and with a one-item list

A mutant is killed when sccache_pin_violations does not reproduce the
baseline violation list for every fixture under
scripts/ci/testdata/sccache_policy, or when sccache_mutation_checks raises.
Exit 0 when no mutant survives.
"""

from __future__ import annotations

import ast
import multiprocessing
import signal
import traceback
from dataclasses import dataclass
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
POLICY = ROOT / "scripts" / "ci" / "test_workflow_policy.py"
FIXTURE_ROOT = ROOT / "scripts" / "ci" / "testdata" / "sccache_policy"
NONEMPTY_RETURN = "mutation"
CHECKER_START = "SCCACHE_ACTION_OWNER"
CHECKER_END = "repo_sccache_action_sites"

_OP_TYPES = {
    "Eq": ast.Eq,
    "NotEq": ast.NotEq,
    "Lt": ast.Lt,
    "LtE": ast.LtE,
    "Gt": ast.Gt,
    "GtE": ast.GtE,
    "In": ast.In,
    "NotIn": ast.NotIn,
    "Is": ast.Is,
    "IsNot": ast.IsNot,
}
_COMPARE_FLIPS = {
    "Eq": ("NotEq",),
    "NotEq": ("Eq",),
    "Lt": ("Gt", "LtE"),
    "LtE": ("GtE", "Lt"),
    "Gt": ("Lt", "GtE"),
    "GtE": ("LtE", "Gt"),
    "In": ("NotIn",),
    "NotIn": ("In",),
    "Is": ("IsNot",),
    "IsNot": ("Is",),
}


@dataclass(frozen=True)
class Spec:
    path: tuple[tuple[str, int | None], ...]
    kind: str
    arg: str
    line: int
    desc: str


def checker_bounds(tree: ast.AST) -> tuple[int, int]:
    start = None
    end = None
    for node in tree.body:
        targets = node.targets if isinstance(node, ast.Assign) else []
        if any(isinstance(target, ast.Name) and target.id == CHECKER_START for target in targets):
            start = node.lineno
        if isinstance(node, ast.FunctionDef) and node.name == CHECKER_END:
            end = node.end_lineno
    if start is None or end is None:
        raise RuntimeError("sccache checker span not found")
    return start, end


def _in_range(node: ast.AST, start: int, end: int) -> bool:
    line = getattr(node, "lineno", None)
    return line is not None and start <= line <= end


def _steps(node: ast.AST) -> list[tuple[str, int | None, ast.AST]]:
    found: list[tuple[str, int | None, ast.AST]] = []
    for field, value in ast.iter_fields(node):
        if isinstance(value, list):
            for index, item in enumerate(value):
                if isinstance(item, ast.AST):
                    found.append((field, index, item))
        elif isinstance(value, ast.AST):
            found.append((field, None, value))
    return found


def _walk_paths(node: ast.AST, prefix: tuple[tuple[str, int | None], ...] = ()):
    yield prefix, node
    for field, index, child in _steps(node):
        yield from _walk_paths(child, prefix + ((field, index),))


def _node_at(tree: ast.AST, path: tuple[tuple[str, int | None], ...]) -> ast.AST:
    node = tree
    for field, index in path:
        value = getattr(node, field)
        node = value if index is None else value[index]
    return node


def _parent_at(tree: ast.AST, path: tuple[tuple[str, int | None], ...]) -> ast.AST:
    return _node_at(tree, path[:-1])


def _replace(parent: ast.AST, field: str, index: int | None, new: ast.AST) -> None:
    if index is None:
        setattr(parent, field, new)
        return
    getattr(parent, field)[index] = new


def _snippet(node: ast.AST) -> str:
    text = ast.unparse(node).replace("\n", " ")
    if len(text) > 90:
        return text[:87] + "..."
    return text


def build_specs(tree: ast.AST) -> list[Spec]:
    start, end = checker_bounds(tree)
    specs: list[Spec] = []
    for path, node in _walk_paths(tree):
        if not _in_range(node, start, end):
            continue
        if isinstance(node, ast.Compare):
            for index, op in enumerate(node.ops):
                name = type(op).__name__
                for flipped in _COMPARE_FLIPS.get(name, ()):
                    specs.append(
                        Spec(path, "compare", f"{index}:{flipped}", node.lineno, f"compare {name} -> {flipped}: {_snippet(node)}")
                    )
        elif isinstance(node, ast.BoolOp):
            swapped = "Or" if isinstance(node.op, ast.And) else "And"
            specs.append(Spec(path, "boolop", swapped, node.lineno, f"bool {type(node.op).__name__} -> {swapped}: {_snippet(node)}"))
            for index in range(len(node.values)):
                specs.append(
                    Spec(path, "drop", str(index), node.lineno, f"drop bool operand {index}: {_snippet(node)}")
                )
        elif isinstance(node, (ast.If, ast.IfExp)):
            specs.append(Spec(path, "if", "", node.lineno, f"invert if: {_snippet(node.test)}"))
        elif isinstance(node, (ast.Continue, ast.Break)):
            specs.append(
                Spec(path, "jump", type(node).__name__, node.lineno, f"swap {type(node).__name__}")
            )
        elif isinstance(node, ast.Constant) and type(node.value) is bool:
            specs.append(
                Spec(path, "bool", str(node.value), node.lineno, f"flip {node.value}")
            )
        elif isinstance(node, ast.Constant) and isinstance(node.value, str) and "0.19.0" in node.value:
            specs.append(
                Spec(path, "version", "", node.lineno, f"version string {node.value!r}")
            )
        elif isinstance(node, ast.Return):
            for arg, label in (("empty", "[]"), ("nonempty", "['mutation']")):
                if node.value is not None and ast.unparse(node.value) == label.replace("'", '"'):
                    continue
                if arg == "empty" and isinstance(node.value, ast.List) and not node.value.elts:
                    continue
                specs.append(
                    Spec(path, "return", arg, node.lineno, f"return -> {label}: {_snippet(node)}")
                )
    specs.sort(key=lambda item: (item.line, item.kind, item.arg, item.desc))
    return specs


def apply_spec(tree: ast.AST, spec: Spec) -> None:
    node = _node_at(tree, spec.path)
    if spec.kind == "compare":
        index_text, op_name = spec.arg.split(":", 1)
        assert isinstance(node, ast.Compare)
        node.ops[int(index_text)] = _OP_TYPES[op_name]()
        return
    if spec.kind == "boolop":
        assert isinstance(node, ast.BoolOp)
        node.op = ast.Or() if spec.arg == "Or" else ast.And()
        return
    if spec.kind == "drop":
        assert isinstance(node, ast.BoolOp)
        index = int(spec.arg)
        kept = [value for value_index, value in enumerate(node.values) if value_index != index]
        if len(kept) == 1:
            field, slot = spec.path[-1]
            _replace(_parent_at(tree, spec.path), field, slot, kept[0])
            return
        node.values = kept
        return
    if spec.kind == "if":
        assert isinstance(node, (ast.If, ast.IfExp))
        inverted = ast.UnaryOp(op=ast.Not(), operand=node.test)
        ast.copy_location(inverted, node.test)
        node.test = inverted
        return
    if spec.kind == "jump":
        field, slot = spec.path[-1]
        replacement: ast.AST
        if spec.arg == "Continue":
            replacement = ast.Break()
        else:
            replacement = ast.Continue()
        ast.copy_location(replacement, node)
        _replace(_parent_at(tree, spec.path), field, slot, replacement)
        return
    if spec.kind == "bool":
        assert isinstance(node, ast.Constant)
        node.value = not node.value
        return
    if spec.kind == "version":
        assert isinstance(node, ast.Constant) and isinstance(node.value, str)
        node.value = node.value.replace("0.19.0", "0.20.0")
        return
    if spec.kind == "return":
        assert isinstance(node, ast.Return)
        if spec.arg == "empty":
            value: ast.AST = ast.List(elts=[], ctx=ast.Load())
        else:
            item = ast.Constant(value=NONEMPTY_RETURN)
            value = ast.List(elts=[item], ctx=ast.Load())
        ast.copy_location(value, node)
        node.value = value
        return
    raise AssertionError(spec.kind)


def _load_fixtures() -> list[tuple[str, str]]:
    fixtures: list[tuple[str, str]] = []
    for kind in ("pass", "fail"):
        directory = FIXTURE_ROOT / kind
        for path in sorted(directory.glob("*.yml")):
            fixtures.append((f"{kind}/{path.name}", path.read_text(encoding="utf-8")))
    fixtures.append(("direct/empty", ""))
    fixtures.append(("direct/null", "null\n"))
    return fixtures


def _exec_tree(tree: ast.AST) -> dict:
    code = compile(tree, str(POLICY), "exec")
    namespace = {"__file__": str(POLICY), "__name__": "sccache_policy_mutant"}
    exec(code, namespace)  # noqa: S102
    return namespace


def _violation_map(function, fixtures: list[tuple[str, str]]) -> dict[str, list[str]]:
    found: dict[str, list[str]] = {}
    for name, text in fixtures:
        found[name] = function(text, path=name)
    return found


def _same(namespace: dict, baseline: dict[str, list[str]], fixtures: list[tuple[str, str]]) -> bool:
    function = namespace["sccache_pin_violations"]
    for name, text in fixtures:
        if function(text, path=name) != baseline[name]:
            return False
    checks = namespace.get("sccache_mutation_checks")
    if checks is not None:
        checks()
    case = namespace["WorkflowPolicyTest"]
    for name in (
        "test_sccache_action_pins_binary_version",
        "test_sccache_pin_scan_reads_workflows_under_root",
    ):
        getattr(case(name), name)()
    return True


class _MutantTimeout(Exception):
    pass


def _alarm(_signum: int, _frame: object) -> None:
    raise _MutantTimeout()


def _evaluate(source: str, spec: Spec, baseline: dict[str, list[str]], fixtures: list[tuple[str, str]]) -> str:
    tree = ast.parse(source)
    apply_spec(tree, spec)
    ast.fix_missing_locations(tree)
    signal.signal(signal.SIGALRM, _alarm)
    signal.alarm(20)
    try:
        namespace = _exec_tree(tree)
        if _same(namespace, baseline, fixtures):
            return "survived"
        return "killed"
    except Exception:
        return "killed"
    finally:
        signal.alarm(0)


def _worker(payload: tuple[list[int], str, list[Spec], dict[str, list[str]], list[tuple[str, str]]]) -> list[tuple[int, str]]:
    indices, source, specs, baseline, fixtures = payload
    results: list[tuple[int, str]] = []
    for index in indices:
        try:
            status = _evaluate(source, specs[index], baseline, fixtures)
        except Exception:
            status = "killed"
        results.append((index, status))
    return results


def main() -> int:
    source = POLICY.read_text(encoding="utf-8")
    parsed = ast.parse(source)
    start, end = checker_bounds(parsed)
    print(f"checker {start}-{end}")
    specs = build_specs(parsed)
    fixtures = _load_fixtures()
    baseline_ns = _exec_tree(ast.parse(source))
    baseline = _violation_map(baseline_ns["sccache_pin_violations"], fixtures)
    checks = baseline_ns.get("sccache_mutation_checks")
    if checks is not None:
        checks()
    baseline_case = baseline_ns["WorkflowPolicyTest"]
    for name in (
        "test_sccache_action_pins_binary_version",
        "test_sccache_pin_scan_reads_workflows_under_root",
    ):
        getattr(baseline_case(name), name)()

    counts: dict[str, int] = {}
    for spec in specs:
        counts[spec.kind] = counts.get(spec.kind, 0) + 1
    print(f"mutants {len(specs)}")
    for kind in sorted(counts):
        print(f"  {kind} {counts[kind]}")

    workers = max(1, min(8, multiprocessing.cpu_count()))
    chunks: list[list[int]] = [[] for _ in range(workers)]
    for index in range(len(specs)):
        chunks[index % workers].append(index)
    payloads = [(chunk, source, specs, baseline, fixtures) for chunk in chunks if chunk]
    survived: list[Spec] = []
    with multiprocessing.Pool(workers) as pool:
        for batch in pool.imap_unordered(_worker, payloads):
            for index, status in batch:
                if status == "survived":
                    survived.append(specs[index])
    survived.sort(key=lambda item: (item.line, item.kind, item.arg, item.desc))
    killed = len(specs) - len(survived)
    print(f"killed {killed}")
    print(f"survived {len(survived)}")
    for spec in survived:
        print(f"SURVIVOR {spec.line} {spec.kind} {spec.arg} {spec.desc}")
    return 0 if not survived else 1


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except Exception:
        traceback.print_exc()
        raise
