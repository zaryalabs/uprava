#!/usr/bin/env python3
"""Validate local Markdown links without requiring network access."""

from __future__ import annotations

import argparse
import re
import tempfile
from pathlib import Path
from urllib.parse import unquote


LINK = re.compile(r"(?<!!)\[[^\]]*\]\(([^)]+)\)|!\[[^\]]*\]\(([^)]+)\)")
FENCE = re.compile(r"^\s*(```|~~~)")


def markdown_files(root: Path) -> list[Path]:
    paths = [root / name for name in ("README.md", "AGENTS.md", "CONTRIBUTING.md")]
    paths.extend((root / "docs").rglob("*.md"))
    return sorted(path for path in paths if path.is_file())


def strip_fenced_code(text: str) -> str:
    output: list[str] = []
    fence: str | None = None
    for line in text.splitlines():
        match = FENCE.match(line)
        if match:
            marker = match.group(1)
            if fence is None:
                fence = marker
            elif marker == fence:
                fence = None
            output.append("")
        elif fence is None:
            output.append(line)
        else:
            output.append("")
    return "\n".join(output)


def local_target(raw: str) -> str | None:
    target = raw.strip()
    if target.startswith("<") and ">" in target:
        target = target[1 : target.index(">")]
    else:
        target = target.split(maxsplit=1)[0]
    if not target or target.startswith(("#", "http://", "https://", "mailto:", "tel:")):
        return None
    return unquote(target.split("#", 1)[0])


def validate(root: Path, files: list[Path] | None = None) -> list[str]:
    errors: list[str] = []
    for source in files or markdown_files(root):
        text = strip_fenced_code(source.read_text(encoding="utf-8"))
        for match in LINK.finditer(text):
            raw = match.group(1) or match.group(2)
            target = local_target(raw)
            if target is None:
                continue
            destination = root / target.lstrip("/") if target.startswith("/") else source.parent / target
            if not destination.resolve().exists():
                line = text.count("\n", 0, match.start()) + 1
                errors.append(f"{source.relative_to(root)}:{line}: broken local link: {raw}")
    return errors


def self_test() -> None:
    with tempfile.TemporaryDirectory() as directory:
        root = Path(directory)
        (root / "docs").mkdir()
        source = root / "README.md"
        source.write_text("[missing](docs/missing.md)\n", encoding="utf-8")
        errors = validate(root, [source])
        if len(errors) != 1 or "docs/missing.md" not in errors[0]:
            raise SystemExit("Markdown link checker failed its broken-link regression")


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--self-test", action="store_true")
    args = parser.parse_args()
    if args.self_test:
        self_test()
        return
    root = Path(__file__).resolve().parent.parent
    errors = validate(root)
    if errors:
        raise SystemExit("\n".join(errors))
    print(f"Markdown links valid ({len(markdown_files(root))} files)")


if __name__ == "__main__":
    main()
