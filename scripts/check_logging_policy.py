#!/usr/bin/env python3
"""Reject sensitive or high-cardinality structured fields in product logs."""

from __future__ import annotations

import re
from pathlib import Path


FORBIDDEN = (
    "secret",
    "credential",
    "token",
    "password",
    "prompt",
    "content",
    "path",
    "node_id",
    "event_id",
    "command_id",
    "terminal_id",
    "session_id",
    "runtime_session_id",
    "placement_id",
    "enrollment_id",
    "display_name",
    "database_url",
    "core_url",
)
MACRO_START = re.compile(r"tracing::(?:trace|debug|info|warn|error)!\(")


def macros(text: str):
    for start in MACRO_START.finditer(text):
        depth = 1
        quote: str | None = None
        escaped = False
        for index in range(start.end(), len(text)):
            char = text[index]
            if quote is not None:
                if escaped:
                    escaped = False
                elif char == "\\":
                    escaped = True
                elif char == quote:
                    quote = None
                continue
            if char in {'"', "'"}:
                quote = char
            elif char == "(":
                depth += 1
            elif char == ")":
                depth -= 1
                if depth == 0:
                    yield start.start(), text[start.end() : index]
                    break


def main() -> None:
    root = Path(__file__).resolve().parent.parent
    sources = (
        root / "crates/uprava-server/src/runtime.rs",
        root / "crates/uprava-server/src/main.rs",
        root / "crates/uprava-node/src/runtime.rs",
        root / "crates/uprava-logging/src/lib.rs",
    )
    errors: list[str] = []
    field_pattern = re.compile(rf"\b({'|'.join(map(re.escape, FORBIDDEN))})\s*=")
    for source in sources:
        text = source.read_text(encoding="utf-8")
        for offset, body in macros(text):
            fields = sorted(set(field_pattern.findall(body)))
            if fields:
                line = text.count("\n", 0, offset) + 1
                errors.append(
                    f"{source.relative_to(root)}:{line}: forbidden log fields: {', '.join(fields)}"
                )
    if errors:
        raise SystemExit("\n".join(errors))
    print("Structured logging cardinality/redaction policy passed")


if __name__ == "__main__":
    main()
