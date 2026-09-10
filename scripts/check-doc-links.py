#!/usr/bin/env python3
"""Check relative Markdown links and heading anchors without fetching URLs."""

import os
import re
import subprocess
import sys

SKIP = ("target/", "refs/", "tmp/", "samples/", "private-data/", "node_modules/")
LINK = re.compile(r"\]\(([^)\s]+)\)")
HEADING = re.compile(r"^#+ (.*)$", re.MULTILINE)


def anchors(path: str) -> set[str]:
    """Return GitHub-style anchors used by the repository's simple headings."""
    with open(path, encoding="utf-8") as source:
        text = source.read()
    result = set()
    for heading in HEADING.findall(text):
        heading = re.sub(r"`", "", heading)
        slug = re.sub(r"[^\w\- ]", "", heading.lower()).strip().replace(" ", "-")
        result.add(slug)
    return result


def main() -> int:
    """Check tracked Markdown files and return a process status."""
    output = subprocess.check_output(
        ["git", "ls-files", "-z", "*.md"], text=True
    )
    problems = 0
    for path in sorted(filter(None, output.split("\0"))):
        if path.startswith(SKIP):
            continue
        with open(path, encoding="utf-8") as source:
            text = source.read()
        for match in LINK.finditer(text):
            link = match.group(1)
            if link.startswith(("http://", "https://", "mailto:")):
                continue
            target, _, anchor = link.partition("#")
            resolved = path if not target else os.path.normpath(
                os.path.join(os.path.dirname(path), target)
            )
            if not os.path.exists(resolved):
                print(f"{path}: broken link {link}")
                problems += 1
            elif anchor and resolved.endswith(".md") and anchor not in anchors(resolved):
                print(f"{path}: missing anchor {link}")
                problems += 1
    print(f"doc links checked, problems={problems}")
    return 1 if problems else 0


if __name__ == "__main__":
    sys.exit(main())
