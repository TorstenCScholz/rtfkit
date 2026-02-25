"""Walk the document IR tree and show to_dict()/to_json()."""

import json

import rtfkit

rtf = r"{\rtf1\ansi {\b Bold text} and {\i italic text}.}"
result = rtfkit.parse(rtf)

# Walk the tree with isinstance()
for block in result.document.blocks:
    if isinstance(block, rtfkit.Paragraph):
        for inline in block.inlines:
            if isinstance(inline, rtfkit.Run):
                attrs = []
                if inline.bold:
                    attrs.append("bold")
                if inline.italic:
                    attrs.append("italic")
                label = f" [{', '.join(attrs)}]" if attrs else ""
                print(f"  Run: {inline.text!r}{label}")

# Serialize to dict and JSON
print("\n--- to_dict() ---")
print(json.dumps(result.document.to_dict(), indent=2))

print("\n--- to_json() ---")
print(result.document.to_json())
