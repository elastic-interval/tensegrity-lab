"""
The Bouwboek as a PDF
=====================

    python3 scripts/bouwboek/pdf.py HeadlessHug-2026-11-12/HeadlessHug-bouwboek-2026-11-12.md

writes the PDF beside the Markdown, with the same name. The Markdown comes
from the handoff package (`export_headless_hug_package`), its figures in
`images/` beside it.

It needs only Python and WeasyPrint's command (`brew install weasyprint`).
The Markdown is turned into HTML here, by hand, which covers what the
template uses: headings, paragraphs, lists, tables, images, *italic*,
**bold** and `code`. The look is `bouwboek.css`, beside this script: A4,
page numbers, a title page and a table of contents with page numbers.
"""

import argparse
import html
import re
import shutil
import subprocess
import sys
from pathlib import Path

STYLESHEET = Path(__file__).with_name("bouwboek.css")


def inline(text):
    """Emphasis, code, images and TE DOEN marks within a line. <br> passes."""
    text = html.escape(text, quote=False).replace("&lt;br&gt;", "<br>")
    text = re.sub(r"`([^`]+)`", r"<code>\1</code>", text)
    text = re.sub(r"!\[([^\]]*)\]\(([^)]+)\)", r'<img alt="\1" src="\2">', text)
    text = re.sub(r"\*\*([^*]+)\*\*", r"<strong>\1</strong>", text)
    text = re.sub(r"\*([^*]+)\*", r"<em>\1</em>", text)
    return text.replace("TE DOEN", '<mark>TE DOEN</mark>')


def cells(line):
    return [cell.strip() for cell in line.strip().strip("|").split("|")]


def table(lines):
    """A pipe table: header, the alignment row, then the body."""
    head, rule, body = cells(lines[0]), cells(lines[1]), [cells(line) for line in lines[2:]]
    align = ["right" if cell.endswith(":") else "left" for cell in rule]

    def row(tag, values):
        return "<tr>" + "".join(
            f'<{tag} class="{a}">{inline(v)}</{tag}>' for v, a in zip(values, align)) + "</tr>"

    header = "" if not any(head) else "<thead>" + row("th", head) + "</thead>"
    return f"<table>{header}<tbody>{''.join(row('td', r) for r in body)}</tbody></table>"


def to_html(markdown):
    """The body, and the headings for the contents as (level, id, text)."""
    out, headings = [], []
    lines = markdown.splitlines()
    i = 0
    while i < len(lines):
        line = lines[i]
        if not line.strip():
            i += 1
        elif line.startswith("#"):
            level = len(line) - len(line.lstrip("#"))
            text = line[level:].strip()
            anchor = f"h{len(headings)}"
            headings.append((level, anchor, text))
            out.append(f'<h{level} id="{anchor}">{inline(text)}</h{level}>')
            i += 1
        elif line.startswith("|"):
            block = []
            while i < len(lines) and lines[i].startswith("|"):
                block.append(lines[i])
                i += 1
            out.append(table(block))
        elif line.startswith("- "):
            items = []
            while i < len(lines) and (lines[i].startswith("- ") or lines[i].startswith("  ")):
                if lines[i].startswith("- "):
                    items.append(lines[i][2:].strip())
                else:
                    items[-1] += " " + lines[i].strip()
                i += 1
            out.append("<ul>" + "".join(f"<li>{inline(item)}</li>" for item in items) + "</ul>")
        elif line.startswith("!["):
            caption = ""
            if i + 2 < len(lines) and lines[i + 2].startswith("*"):
                caption = lines[i + 2].strip().strip("*")
            kind = "part" if "](images/onderdeel-" in line else "view"
            out.append(f'<figure class="{kind}">{inline(line.strip())}<figcaption>{inline(caption)}</figcaption></figure>')
            i += 3 if caption else 1
        else:
            paragraph = []
            while i < len(lines) and lines[i].strip() and not lines[i].startswith(("#", "|", "- ", "![")):
                paragraph.append(lines[i].strip())
                i += 1
            out.append(f"<p>{inline(' '.join(paragraph))}</p>")
    return out, headings


def document(markdown):
    """Title page (the title and the table under it), contents, then the chapters."""
    body, headings = to_html(markdown)
    first_chapter = next(n for n, part in enumerate(body) if part.startswith("<h2"))
    contents = "".join(
        f'<li class="level{level}"><a href="#{anchor}">{inline(text)}</a></li>'
        for level, anchor, text in headings if level in (2, 3))
    title = re.sub(r"<[^>]+>", "", body[0])
    return f"""<!doctype html>
<html lang="nl"><head><meta charset="utf-8"><title>{title}</title>
<link rel="stylesheet" href="{STYLESHEET.resolve().as_uri()}"></head>
<body>
<section class="cover">{''.join(body[:first_chapter])}</section>
<nav class="contents"><h2 class="unnumbered">Inhoudsopgave</h2><ul>{contents}</ul></nav>
{''.join(body[first_chapter:])}
</body></html>
"""


def main():
    parser = argparse.ArgumentParser(description="The Bouwboek Markdown as a PDF, beside it.")
    parser.add_argument("markdown", type=Path)
    args = parser.parse_args()
    weasyprint = shutil.which("weasyprint")
    if not weasyprint:
        sys.exit("weasyprint not found: brew install weasyprint")
    source = args.markdown.resolve()
    page = source.with_suffix(".html")
    page.write_text(document(source.read_text(encoding="utf-8")), encoding="utf-8")
    pdf = source.with_suffix(".pdf")
    try:
        subprocess.run([weasyprint, str(page), str(pdf)], check=True)
    finally:
        page.unlink()
    print(pdf)


if __name__ == "__main__":
    main()
