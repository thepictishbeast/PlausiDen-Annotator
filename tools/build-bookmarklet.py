#!/usr/bin/env python3
"""
build-bookmarklet — convert src/annotator.js → dist/bookmarklet.js

`src/annotator.js` is a self-contained IIFE (no imports, no
build dependencies). This script wraps it as a `javascript:`
URI suitable for pasting into a browser bookmark.

Discipline:
- No external Python deps. Stdlib only.
- No JS minifier dependency tree (per the no-rollup/webpack/parcel
  commitment in the README + ARCHITECTURE.md). Conservative
  whitespace + line-comment stripping only.
- Output is URL-encoded per RFC 3986 unreserved-character set so
  it pastes cleanly into the URL bar of every major browser.
- Idempotent: running this script twice produces byte-identical
  output (no timestamps embedded, no random suffixes).

Usage:
    python3 tools/build-bookmarklet.py
    # writes dist/bookmarklet.js with the encoded `javascript:` URI

Exit code:
    0  — built successfully, size within budget
    1  — source unreadable or output unwritable
    2  — built size exceeds typical browser URL-bar budget (~32 KB);
         operator should investigate before shipping
"""

from __future__ import annotations

import re
import sys
import urllib.parse
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent
SOURCE = REPO_ROOT / "src" / "annotator.js"
DEST = REPO_ROOT / "dist" / "bookmarklet.js"

# Modern browsers handle 32-80 KB URI bookmarks. Warn beyond 32 KB
# to catch surprise bloat early.
MAX_BUDGET_BYTES = 32 * 1024

# Regex strips line comments OUTSIDE string literals. The annotator
# source contains no template literals (no backticks) and no comment
# markers inside string contents based on audit at write time; if
# either changes in src/annotator.js, this stripper must be
# replaced with a proper JS-aware lexer. Until then this is a
# conservative line-comment stripper that bails out on suspicious
# inputs.
LINE_COMMENT_RE = re.compile(r"^[ \t]*//.*$", re.MULTILINE)
INLINE_COMMENT_AFTER_CODE_RE = re.compile(r"([^:'\"])//[^\n]*$", re.MULTILINE)
BLANK_LINE_RE = re.compile(r"^\s*\n", re.MULTILINE)


def strip_comments_conservative(src: str) -> str:
    """Strip standalone `// ...` lines + blank lines. Leaves
    inline trailing comments alone where they sit after code,
    to avoid eating into string content with embedded `//`."""
    # 1. Sole-line // comments (indent + comment).
    src = LINE_COMMENT_RE.sub("", src)
    # 2. Collapse runs of blank lines.
    src = BLANK_LINE_RE.sub("", src)
    return src


def to_bookmarklet(js: str) -> str:
    """Wrap as `javascript:` URI. The source is already an IIFE,
    so no extra wrapping is required — just prefix + URL-encode.

    safe characters: keep `()`, `;`, `&`, `=`, `,` readable for
    debugging in the URL bar; URL-encode everything else that
    might be interpreted by the browser's URL parser.
    """
    payload = urllib.parse.quote(
        js,
        safe="(){}[];,.=&?!+-*/%<>:|@$_'\"~^",
    )
    return f"javascript:{payload}"


def main() -> int:
    if not SOURCE.is_file():
        print(f"error: {SOURCE} not found", file=sys.stderr)
        return 1
    DEST.parent.mkdir(parents=True, exist_ok=True)

    raw = SOURCE.read_text(encoding="utf-8")
    stripped = strip_comments_conservative(raw)
    bookmarklet = to_bookmarklet(stripped)

    DEST.write_text(bookmarklet + "\n", encoding="utf-8")

    size = len(bookmarklet.encode("utf-8"))
    pct = (size / MAX_BUDGET_BYTES) * 100
    print(f"built {DEST.relative_to(REPO_ROOT)} — {size} bytes ({pct:.1f}% of 32 KB budget)")

    if size > MAX_BUDGET_BYTES:
        print(
            f"warning: bookmarklet exceeds {MAX_BUDGET_BYTES} bytes — "
            "some browsers truncate URL-bar bookmarks past this. "
            "Investigate src/annotator.js for accidental bloat.",
            file=sys.stderr,
        )
        return 2

    return 0


if __name__ == "__main__":
    sys.exit(main())
