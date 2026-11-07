#!/usr/bin/env python3
"""Generate the public project todo.txt from docs/BACKLOG.md (stdlib only)."""

import argparse
from pathlib import Path
import re

ROOT = Path(__file__).resolve().parent.parent
SOURCE = ROOT / "docs" / "BACKLOG.md"
TARGET = ROOT / "todo.txt"
CHECKBOX = re.compile(r"^- \[([ xX])\] (.+)$")


def plain_text(text):
    text = re.sub(r"\[([^\]]+)\]\(([^)]+)\)", r"\1 (\2)", text)
    return " ".join(text.replace("**", "").replace("`", "").split())


def render(markdown):
    tasks = []
    context = None
    found = 0
    for line in markdown.splitlines():
        if line.startswith("## "):
            if line == "## Current work: parity and validation":
                context = "@parity"
            elif line == "## Deferred ideas":
                context = "@deferred"
            else:
                context = None
            continue
        match = CHECKBOX.match(line)
        if not match:
            continue
        if context is None:
            raise ValueError("A backlog item is outside a recognized planning section")
        found += 1
        if match[1].lower() == "x":
            continue
        title = re.match(r"\(([A-Z])\)\s+\*\*(.+?)\*\*(?:\s|$)", match[2])
        if not title:
            raise ValueError("Each pending backlog item needs an (A)-(Z) priority and concise bold action title")
        action = plain_text(title[2]).rstrip(".")
        tasks.append(f"({title[1]}) {action} +todotxt.rs {context}")
    if not found:
        raise ValueError("No backlog checkboxes found; refusing to generate a task list")
    tasks.sort(key=lambda task: task[1])  # Stable within each priority.
    return "".join(task + "\n" for task in tasks)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true", help="Report stale output without writing")
    args = parser.parse_args()
    expected = render(SOURCE.read_text(encoding="utf-8"))
    if args.check:
        actual = TARGET.read_text(encoding="utf-8") if TARGET.exists() else None
        if actual != expected:
            print("todo.txt is out of date. Run: python scripts/sync-backlog.py")
            return 1
        print(f"todo.txt is synchronized ({len(expected.splitlines())} pending tasks).")
    else:
        TARGET.write_bytes(expected.encode("utf-8"))
        print(f"Generated todo.txt ({len(expected.splitlines())} pending tasks).")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
