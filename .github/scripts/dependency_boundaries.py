#!/usr/bin/env python3
"""The crate dependency-boundary guard (AGENTS §1; item L32).

Rules enforced over the workspace's runtime dependency graph (the
`[dependencies]` sections of `crates/*/Cargo.toml`):

1. **Acyclic** — the datacraft crate graph has no cycles.
2. **Direction** — only the binary composition roots (`datacraft-ui`
   with its `src/bin/*` targets, `datacraft-cli`, `datacraft-mcp`) may
   name `datacraft-storage` or `datacraft-csv`; library crates reach
   storage exclusively through core commands.

`--self-test` runs the check functions against synthetic manifest sets
(a clean graph, a direction violation, a cycle) and fails if any
violation goes undetected — the guard validates itself on every CI run
before it judges the real tree.

Usage: `dependency_boundaries.py [--self-test | <workspace-root>]`
"""

import sys
import tomllib
from pathlib import Path

# AGENTS §1: the binary composition roots are the only crates allowed to
# name the composite or the out-of-tree driver.
COMPOSITION_ROOTS = {"datacraft-ui", "datacraft-cli", "datacraft-mcp"}
GATED_DEPS = {"datacraft-storage", "datacraft-csv"}


def read_graph(root: Path) -> dict[str, list[str]]:
    """Crate name -> the datacraft crates its [dependencies] name.

    `workspace = true` inheritance counts: the member manifest names the
    dependency either way, and the resolved edge is what the graph is
    about. `[workspace.dependencies]` itself is a version registry, not
    an edge, and is deliberately ignored.
    """
    graph: dict[str, list[str]] = {}
    for manifest in sorted(root.glob("crates/*/Cargo.toml")):
        with manifest.open("rb") as handle:
            data = tomllib.load(handle)
        name = data["package"]["name"]
        graph[name] = sorted(
            dep
            for dep in data.get("dependencies", {})
            if dep.startswith("datacraft-")
        )
    return graph


def find_cycles(graph: dict[str, list[str]]) -> list[list[str]]:
    """Every cycle in the graph, deduplicated by node set."""
    cycles: list[list[str]] = []
    seen: set[frozenset[str]] = set()
    for start in sorted(graph):
        stack: list[tuple[str, list[str]]] = [(start, [start])]
        while stack:
            node, path = stack.pop()
            for dep in graph.get(node, ()):
                if dep == start:
                    key = frozenset(path)
                    if key not in seen:
                        seen.add(key)
                        cycles.append(path)
                elif dep in graph and dep not in path:
                    stack.append((dep, path + [dep]))
    return cycles


def find_direction_violations(graph: dict[str, list[str]]) -> list[str]:
    """Library crates naming a gated dep, with the precise reason."""
    violations = []
    for crate, deps in sorted(graph.items()):
        if crate in COMPOSITION_ROOTS:
            continue
        for dep in deps:
            if dep in GATED_DEPS:
                violations.append(
                    f"{crate} names {dep} — only the binary composition "
                    f"roots ({', '.join(sorted(COMPOSITION_ROOTS))}) may "
                    f"(AGENTS §1)"
                )
    return violations


def check(root: Path) -> list[str]:
    """All boundary problems of the workspace at `root`."""
    graph = read_graph(root)
    problems = [
        f"cycle: {' -> '.join(cycle + [cycle[0]])}"
        for cycle in find_cycles(graph)
    ]
    problems.extend(find_direction_violations(graph))
    return problems


def self_test() -> None:
    """The guard must catch what it exists for."""
    clean = {
        "datacraft-core": [],
        "datacraft-jet": ["datacraft-core"],
        "datacraft-csv": ["datacraft-core"],
        "datacraft-storage": ["datacraft-core", "datacraft-jet"],
        "datacraft-cli": ["datacraft-core", "datacraft-storage"],
        "datacraft-ui": ["datacraft-core", "datacraft-storage"],
        "datacraft-mcp": ["datacraft-core", "datacraft-storage"],
    }
    assert find_direction_violations(clean) == [], "the clean graph passes"
    assert find_cycles(clean) == [], "the clean graph is acyclic"

    broken_direction = dict(clean)
    broken_direction["datacraft-jet"] = ["datacraft-core", "datacraft-csv"]
    violations = find_direction_violations(broken_direction)
    assert any("datacraft-jet names datacraft-csv" in line for line in violations), (
        "a library naming datacraft-csv must be caught"
    )

    cyclic = {"datacraft-core": ["datacraft-jet"], "datacraft-jet": ["datacraft-core"]}
    assert find_cycles(cyclic), "a dependency cycle must be caught"
    print("self-test ok: the clean graph passes; direction violations and cycles are caught")


def main(argv: list[str]) -> int:
    if "--self-test" in argv:
        self_test()
        return 0
    root = Path(argv[1]) if len(argv) > 1 else Path(".")
    problems = check(root)
    if problems:
        for problem in problems:
            print(f"boundary violation: {problem}", file=sys.stderr)
        return 1
    graph = read_graph(root)
    edges = sum(len(deps) for deps in graph.values())
    print(f"boundaries ok: {len(graph)} crates, {edges} datacraft edges, acyclic")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
