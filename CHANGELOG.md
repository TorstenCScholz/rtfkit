# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/),
and this project adheres to [Semantic Versioning](https://semver.org/).

## [1.0.0] - 2026-03-17

First stable release of `rtfkit`.

`rtfkit` 1.0.0 establishes the project as a production-ready open-source RTF conversion toolkit with a unified CLI, Python bindings, and support for DOCX, HTML, PDF, IR JSON, and conversion reports.

### Highlights

- Stable command-line workflow for converting RTF to DOCX, HTML, and PDF
- In-process PDF generation with no external PDF CLI dependency
- Python bindings for parsing and conversion workflows
- Deterministic output and warning behavior suitable for automation and testing
- Parser safety limits for input size, nesting depth, warnings, tables, and images
- Broad support for real-world RTF content including lists, tables, fields, notes, headers/footers, and embedded images

### Added

- DOCX output as the default editable export format
- Semantic HTML5 output via `--to html`
- In-process PDF output via `--to pdf`
- IR JSON emission via `--emit-ir`
- Text and JSON conversion reports with warnings and statistics
- Text styling support for bold, italic, underline, strikethrough, caps, small caps fallback, font families, font sizes, foreground colors, highlights, and shading
- List support including bullet lists, decimal lists, and nested lists
- Table support including merges, borders, alignment, row/cell shading, and nested tables
- Hyperlinks, bookmark anchors, page fields, TOC markers, semantic cross-references, document-property fields, and merge-field fallback rendering
- Headers, footers, footnotes, and endnotes
- Embedded PNG and JPEG image support
- Style profiles for cross-format HTML, PDF, and DOCX output
- Golden tests, determinism tests, limits tests, contract tests, and integration coverage
- Cross-platform release automation and packaged binary artifacts

### Changed

- PDF generation now runs fully in process instead of relying on an external Typst CLI
- The project documentation now presents `rtfkit` as a stable 1.0.0 release with a current feature-focused support story

### Removed

- External PDF backend selection flags and intermediate-file workflow from the active CLI contract

### Notes

- This release consolidates the work that previously landed across pre-1.0 development milestones into a single stable public release story.
- For the exact currently supported feature set and known limitations, see `README.md`, `docs/rtf-feature-overview.md`, and `docs/feature-support.md`.

## Earlier development history

Before `1.0.0`, `rtfkit` evolved through a series of pre-1.0 internal milestones that introduced the core parser, DOCX output, list support, table support, HTML output, PDF output, fonts and colors, highlights and shading, hyperlinks and fields, images, document structure features, and release automation.

Those incremental milestones are intentionally collapsed here so the public changelog reflects the stable release story rather than the internal implementation sequence that led to it.
