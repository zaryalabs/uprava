#!/usr/bin/env python3
"""Keep Core and Node composition roots small and their modules discoverable."""

from __future__ import annotations

from pathlib import Path


ROOT = Path(__file__).resolve().parent.parent

COMPOSITION_LIMITS = {
    Path("crates/uprava-server/src/runtime.rs"): 300,
    Path("crates/uprava-node/src/runtime.rs"): 220,
}

REQUIRED_MODULES = (
    Path("crates/uprava-server/src/runtime/application/coordination.rs"),
    Path("crates/uprava-server/src/runtime/application/projection.rs"),
    Path("crates/uprava-server/src/runtime/application/scheduling.rs"),
    Path("crates/uprava-server/src/runtime/application/session.rs"),
    Path("crates/uprava-server/src/runtime/application/workspace.rs"),
    Path("crates/uprava-server/src/runtime/transport/http.rs"),
    Path("crates/uprava-server/src/runtime/transport/live.rs"),
    Path("crates/uprava-server/src/runtime/transport/node.rs"),
    Path("crates/uprava-server/src/persistence/event.rs"),
    Path("crates/uprava-server/src/persistence/migrations.rs"),
    Path("crates/uprava-server/src/persistence/node.rs"),
    Path("crates/uprava-node/src/runtime/application/dispatch.rs"),
    Path("crates/uprava-node/src/runtime/application/execution.rs"),
    Path("crates/uprava-node/src/runtime/persistence/state.rs"),
    Path("crates/uprava-node/src/runtime/transport/control.rs"),
    Path("crates/uprava-node/src/runtime/transport/enrollment.rs"),
    Path("crates/uprava-node/src/runtime/provider.rs"),
    Path("crates/uprava-node/src/runtime/terminal.rs"),
    Path("crates/uprava-node/src/runtime/workspace.rs"),
)

PERSISTENCE_FORBIDDEN = {
    Path("crates/uprava-server/src/persistence"): (
        "axum::",
        "WebSocket",
        "WebSocketUpgrade",
    ),
    Path("crates/uprava-node/src/runtime/persistence"): (
        "reqwest::",
        "tokio_tungstenite",
        "pty_process",
        "TokioCommand",
    ),
}


def line_count(path: Path) -> int:
    return len(path.read_text(encoding="utf-8").splitlines())


def main() -> None:
    errors: list[str] = []

    for relative_path, limit in COMPOSITION_LIMITS.items():
        path = ROOT / relative_path
        count = line_count(path)
        if count > limit:
            errors.append(f"{relative_path}: {count} lines exceeds composition limit {limit}")

    for relative_path in REQUIRED_MODULES:
        path = ROOT / relative_path
        if not path.is_file():
            errors.append(f"{relative_path}: required runtime boundary module is missing")

    for relative_root, forbidden_values in PERSISTENCE_FORBIDDEN.items():
        for path in sorted((ROOT / relative_root).rglob("*.rs")):
            text = path.read_text(encoding="utf-8")
            for forbidden in forbidden_values:
                if forbidden in text:
                    errors.append(
                        f"{path.relative_to(ROOT)}: persistence boundary contains `{forbidden}`"
                    )

    if errors:
        raise SystemExit("\n".join(errors))

    print("Runtime composition and persistence boundaries valid")


if __name__ == "__main__":
    main()
