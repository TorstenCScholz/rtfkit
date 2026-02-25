"""Convert an RTF file to HTML, DOCX, and PDF."""

import sys

import rtfkit

rtf_path = sys.argv[1] if len(sys.argv) > 1 else "../../fixtures/text_simple_paragraph.rtf"
with open(rtf_path) as f:
    rtf_content = f.read()

result = rtfkit.parse(rtf_content)
print(
    f"Parsed: {len(result.document)} blocks, "
    f"{result.report.stats.paragraph_count} paragraphs, "
    f"{len(result.report.warnings)} warnings"
)

# HTML
html = rtfkit.to_html(result.document, style_profile="report")
with open("output.html", "w") as f:
    f.write(html)

# DOCX
rtfkit.to_docx_file(result.document, "output.docx")

# PDF
pdf_bytes = rtfkit.to_pdf(result.document, page_size="a4")
with open("output.pdf", "wb") as f:
    f.write(pdf_bytes)

print("Wrote output.html, output.docx, output.pdf")
