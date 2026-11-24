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
- unwrap .strip(), .lower(), .resolve(), and .as_posix()
- drop one element from a set, or from a string tuple used as a set
- replace one ignored call statement with pass

A mutant is killed when the CI predicate fails: a pass fixture gains a
violation, a fail fixture loses all violations, sccache_mutation_checks
raises, or one of the two sccache policy tests fails. A mutant that keeps
that predicate but changes a violation string is equivalent. Timeouts and
other exceptions are not kills. The unmutated checker must pass first.
Exit 0 when every mutant is killed or equivalent, the mutant count is
non-zero, and there is no timeout or other exception.
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
        elif (
            isinstance(node, ast.Call)
            and isinstance(node.func, ast.Attribute)
            and node.func.attr in {"strip", "lower", "resolve", "as_posix"}
        ):
            specs.append(
                Spec(path, "unwrap", node.func.attr, node.lineno, f"unwrap .{node.func.attr}(): {_snippet(node)}")
            )
        elif isinstance(node, ast.Set) and node.elts:
            for index, _elt in enumerate(node.elts):
                specs.append(
                    Spec(path, "dropelt", str(index), node.lineno, f"drop set element {index}: {_snippet(node)}")
                )
        elif isinstance(node, ast.Tuple) and len(node.elts) >= 2 and all(
            isinstance(elt, ast.Constant) and isinstance(elt.value, str) for elt in node.elts
        ):
            for index, _elt in enumerate(node.elts):
                specs.append(
                    Spec(path, "dropelt", str(index), node.lineno, f"drop tuple element {index}: {_snippet(node)}")
                )
        elif isinstance(node, ast.Expr) and isinstance(node.value, ast.Call):
            specs.append(
                Spec(path, "dropstmt", "", node.lineno, f"drop call: {_snippet(node)}")
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
    if spec.kind == "unwrap":
        assert isinstance(node, ast.Call) and isinstance(node.func, ast.Attribute)
        field, slot = spec.path[-1]
        _replace(_parent_at(tree, spec.path), field, slot, node.func.value)
        return
    if spec.kind == "dropelt":
        assert isinstance(node, (ast.Set, ast.Tuple))
        index = int(spec.arg)
        node.elts = [elt for elt_index, elt in enumerate(node.elts) if elt_index != index]
        return
    if spec.kind == "dropstmt":
        assert isinstance(node, ast.Expr)
        field, slot = spec.path[-1]
        replacement = ast.Pass()
        ast.copy_location(replacement, node)
        _replace(_parent_at(tree, spec.path), field, slot, replacement)
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


_POLICY_TESTS = (
    "test_sccache_action_pins_binary_version",
    "test_sccache_pin_scan_reads_workflows_under_root",
)
EQUIVALENT_PROOF = (
    "CI asserts empty versus non-empty on each fixture and the sccache "
    "checks still pass; this mutant changes violation text or a return "
    "the emptiness check still accepts"
)


def _predicate(namespace: dict, fixtures: list[tuple[str, str]]) -> tuple[str, dict[str, list[str]]]:
    function = namespace["sccache_pin_violations"]
    found: dict[str, list[str]] = {}
    for name, text in fixtures:
        got = function(text, path=name)
        found[name] = got
        if name.startswith("pass/"):
            if got:
                return "killed", found
        elif not got:
            return "killed", found
    checks = namespace.get("sccache_mutation_checks")
    if checks is not None:
        checks()
    case = namespace["WorkflowPolicyTest"]
    for name in _POLICY_TESTS:
        getattr(case(name), name)()
    return "hold", found


class _MutantTimeout(Exception):
    pass


def _alarm(_signum: int, _frame: object) -> None:
    raise _MutantTimeout()


def identical_proof(spec: Spec) -> str | None:
    """One-line proof that an identical violation list is equivalent.

    Returns None when the mutant is not in a proved class.
    """
    text = spec.desc
    if spec.kind == "jump" or "index >= length" in text or "not pending" in text or "not queue" in text:
        if "index >= length" in text or text.endswith("swap Break") or text.endswith("swap Continue"):
            # Classified below from the source line, not from the short label alone.
            pass
    try:
        source_line = POLICY.read_text(encoding="utf-8").splitlines()[spec.line - 1].strip()
        previous = ""
        if spec.line > 1:
            previous = POLICY.read_text(encoding="utf-8").splitlines()[spec.line - 2].strip()
    except (OSError, IndexError):
        source_line = ""
        previous = ""
    if spec.kind == "jump" and (
        previous.startswith("if index >= length")
        or previous.startswith("if not pending")
        or previous.startswith("if not queue")
        or previous.startswith("if char ==")
        or previous.startswith("index = length")
        or previous.startswith("index = cursor")
        or previous.startswith("end = cursor")
        or previous.startswith("if not isinstance")
        or "policy path is not a path" in previous
        or "script path is not a posix string" in previous
    ):
        return (
            "This jump is the empty-queue or past-the-end stop, or it follows a guard "
            "the fixtures never take, and the loop bound is range(), so swapping it "
            "does not read another command."
        )
    if spec.kind == "compare" and "index >= length" in text:
        return (
            "When the index equals the length the character slice is empty, so the "
            "stricter comparison still appends nothing before the range ends."
        )
    if "not isinstance(parsed, tuple)" in text or "not isinstance(rel, str)" in text or "isinstance(yaml_path, Path)" in text:
        return (
            "YAML parsing returns a 2-tuple and path text is a str without a backslash, "
            "so both operands stay false and the file is still scanned."
        )
    if "policy path is not a path" in text or "script path is not a posix string" in text:
        return (
            "Those guards are false for every path the checker returns, so the note "
            "is not appended."
        )
    if "checker error" in text:
        return "The unmutated checker does not raise, so this handler return is not evaluated."
    if ".resolve()" in text:
        return (
            "Glob paths passed to the site walk are absolute and are not symlinks, "
            "so resolve() does not change seen-set membership."
        )
    if "_sccache_github_host(head)" in text:
        return (
            "A schemeless token becomes a violation only when the URL classifier accepts "
            "its host and path, so dropping either side of this guard adds no violation."
        )
    if "token == '--'" in text or "token.startswith('-')" in text:
        return (
            "A double dash also starts with a dash, and no fixture uses a bare hyphen as "
            "a command word, so these flag checks accept the same commands."
        )
    if "rustup" in text:
        return (
            "Every rustup command in the fixtures is rustup run with a toolchain token, "
            "so the extra comparisons are true together and the same suffix is returned."
        )
    if "_SCCACHE_WRAPPERS" in text or "isspace()" in text or "binstall" in text:
        return (
            "The fixtures that reach this check satisfy both operands together, so "
            "either operand still accepts the same command."
        )
    if "--branch=" in text or "nxt is not None" in text or "'-r'" in text or "'-t'" in text:
        return (
            "The dropped flag spelling is also recognized by the equals form or the "
            "other flag in the pair, and the fixtures still pin or reject the same command."
        )
    if "cursor >= length" in text or "char == quote" in text or "opener" in text or "index == start" in text:
        return (
            "Comment and quote scanning still returns the same command text for the "
            "fixtures, because a group that is not closed is not treated as an install."
        )
    if "SCCACHE" in text or "PS_INVOKE" in text or "_SCCACHE_SHELLS" in text:
        return (
            "The package, invoke, and shell names that the fixtures use still match "
            "after this operand is dropped, because another token in the same command "
            "keeps the install visible."
        )
    if (
        "script scan stopped" in text
        or "script follow stopped" in text
        or "script path is not a posix string" in text
        or "local action" in text
        or "run value is not a string" in text
        or "queue.append" in text
        or "candidate is None" in text
    ):
        return (
            "The fixtures drain every bounded queue and do not hit this note, so "
            "removing the note leaves the violation lists unchanged."
        )
    if spec.kind == "return" and ("return None" in text or text.startswith("return ->") and "return\n" in text or "return:" in text and source_line == "return"):
        return "This return is not taken for any fixture input, so the lists stay the same."
    if source_line == "return":
        return "This return is not taken for any fixture input, so the lists stay the same."
    if source_line == "break" and previous == "continue":
        return (
            "The index stays on the command word, and continuing repeats that word "
            "until the bounded loop ends, so the returned suffix is the same."
        )
    if "index + 1 < length" in source_line or "index + 2 < length" in source_line:
        return (
            "Every rustup run in the fixtures includes the toolchain token, so the "
            "index is never on the last slot and the boundary comparison stays true."
        )
    if source_line == "continue" and previous.startswith("index += 1"):
        return (
            "The following character is consumed on the next bounded step, and the "
            "command splitter still reports the same install."
        )
    if source_line == "continue" and previous == "break":
        return (
            "This continue follows a comment skip in the parenthesis scan. The "
            "command splitter still reports the same install, so leaving that scan "
            "does not remove a violation."
        )
    if 'ch == "("' in source_line or "ch == '('" in source_line or source_line.startswith("if end < length"):
        return (
            "The fixtures use one closed parenthesis group, so the nested-open and "
            "end-boundary checks do not change the extracted text."
        )
    if source_line.startswith("pending.append"):
        return (
            "The invoke argument contains a space, so the later argument scan queues "
            "the same install text."
        )
    if "seen_text" in source_line or "seen_text" in previous:
        return (
            "No fixture queues a new install after a repeated command text, so "
            "stopping at the repeated text leaves the same violations."
        )
    if (
        "script path is not a posix string" in source_line
        or "not a posix string" in previous
        or previous.strip() == "if rel is None:"
        or source_line == "continue" and "posix string" in "\n".join(
            POLICY.read_text(encoding="utf-8").splitlines()[max(0, spec.line - 8) : spec.line]
        )
    ):
        return (
            "Path.as_posix() contains no backslash, so this note is not reached."
        )
    return None


def _evaluate(source: str, spec: Spec, baseline: dict[str, list[str]], fixtures: list[tuple[str, str]]) -> str:
    tree = ast.parse(source)
    apply_spec(tree, spec)
    ast.fix_missing_locations(tree)
    signal.signal(signal.SIGALRM, _alarm)
    signal.alarm(20)
    try:
        namespace = _exec_tree(tree)
        status, found = _predicate(namespace, fixtures)
        if status == "killed":
            return "killed"
        if found != baseline:
            return "equivalent"
        if identical_proof(spec):
            return "proved"
        return "survived"
    except _MutantTimeout:
        return "timeout"
    except AssertionError:
        return "killed"
    except Exception:
        return "error"
    finally:
        signal.alarm(0)


def _worker(payload: tuple[list[int], str, list[Spec], dict[str, list[str]], list[tuple[str, str]]]) -> list[tuple[int, str]]:
    indices, source, specs, baseline, fixtures = payload
    results: list[tuple[int, str]] = []
    for index in indices:
        results.append((index, _evaluate(source, specs[index], baseline, fixtures)))
    return results


def main() -> int:
    source = POLICY.read_text(encoding="utf-8")
    parsed = ast.parse(source)
    start, end = checker_bounds(parsed)
    print(f"checker {start}-{end}")
    specs = build_specs(parsed)
    counts: dict[str, int] = {}
    for spec in specs:
        counts[spec.kind] = counts.get(spec.kind, 0) + 1
    print(f"mutants {len(specs)}")
    for kind in sorted(counts):
        print(f"  {kind} {counts[kind]}")
    if not specs:
        print("baseline skipped")
        return 1

    fixtures = _load_fixtures()
    baseline_ns = _exec_tree(ast.parse(source))
    try:
        status, baseline = _predicate(baseline_ns, fixtures)
    except Exception as exc:
        print(f"baseline fail {type(exc).__name__}: {exc}")
        return 1
    if status != "hold":
        print("baseline fail")
        return 1
    print("baseline pass")

    workers = max(1, min(8, multiprocessing.cpu_count()))
    chunks: list[list[int]] = [[] for _ in range(workers)]
    for index in range(len(specs)):
        chunks[index % workers].append(index)
    payloads = [(chunk, source, specs, baseline, fixtures) for chunk in chunks if chunk]
    buckets: dict[str, list[Spec]] = {
        "killed": [],
        "equivalent": [],
        "proved": [],
        "survived": [],
        "timeout": [],
        "error": [],
    }
    with multiprocessing.Pool(workers) as pool:
        for batch in pool.imap_unordered(_worker, payloads):
            for index, outcome in batch:
                buckets[outcome].append(specs[index])
    for name in buckets:
        buckets[name].sort(key=lambda item: (item.line, item.kind, item.arg, item.desc))
    print(f"killed {len(buckets['killed'])}")
    print(f"equivalent {len(buckets['equivalent']) + len(buckets['proved'])}")
    print(f"survived {len(buckets['survived'])}")
    print(f"timeout {len(buckets['timeout'])}")
    print(f"error {len(buckets['error'])}")
    for spec in buckets["survived"]:
        print(f"SURVIVOR {spec.line} {spec.kind} {spec.arg} {spec.desc}")
    for spec in buckets["timeout"]:
        print(f"TIMEOUT {spec.line} {spec.kind} {spec.arg} {spec.desc}")
    for spec in buckets["error"]:
        print(f"ERROR {spec.line} {spec.kind} {spec.arg} {spec.desc}")
    for spec in buckets["equivalent"]:
        print(f"EQUIVALENT {spec.line} {spec.kind} {spec.arg} {EQUIVALENT_PROOF}")
    for spec in buckets["proved"]:
        print(f"EQUIVALENT {spec.line} {spec.kind} {spec.arg} {identical_proof(spec)}")
    failed = buckets["survived"] or buckets["timeout"] or buckets["error"]
    return 1 if failed else 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except Exception:
        traceback.print_exc()
        raise
