# CSS Polish Phase - Implementation Plan

## Executive Summary

This document provides a detailed implementation plan for the CSS Polish phase as specified in `docs/specs/PHASE_CSS_POLISH.md`. The phase improves visual quality of HTML output while preserving existing parser, warning, strict-mode, and determinism contracts.

## Current State Analysis

### HTML Writer Options (`crates/rtfkit-html/src/options.rs`)

Current structure:
```rust
pub struct HtmlWriterOptions {
    pub emit_document_wrapper: bool,
    pub include_default_css: bool,
}
```

**Gaps identified:**
- No CSS mode enum (default/none/external)
- No support for custom CSS file paths
- Boolean `include_default_css` needs to become an enum

### CSS Handling (`crates/rtfkit-html/src/style.rs`)

Current implementation:
- [`default_stylesheet()`](crates/rtfkit-html/src/style.rs:79) returns a minimal static CSS string
- No CSS custom properties (tokens)
- No print styles
- Classes defined: `rtf-align-*`, `rtf-u`, `rtf-table`, `rtf-valign-*`

Current CSS (minified):
```css
.rtf-align-left{text-align:left}
.rtf-align-center{text-align:center}
.rtf-align-right{text-align:right}
.rtf-align-justify{text-align:justify}
.rtf-u{text-decoration:underline}
.rtf-table{border-collapse:collapse}
.rtf-valign-top{vertical-align:top}
.rtf-valign-middle{vertical-align:middle}
.rtf-valign-bottom{vertical-align:bottom}
```

### Writer Implementation (`crates/rtfkit-html/src/writer.rs`)

Current flow:
1. [`emit_document_start()`](crates/rtfkit-html/src/writer.rs:76) conditionally embeds CSS
2. CSS embedded via `<style>` tag if `include_default_css` is true
3. No support for appending custom CSS

### CLI Implementation (`crates/rtfkit-cli/src/main.rs`)

Current state:
- `--to html` flag for HTML output
- Uses `HtmlWriterOptions::default()` unconditionally
- No `--html-css` or `--html-css-file` flags

### Current HTML Output Structure

From golden files analysis:
```html
<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<style>/* minimal CSS */</style>
</head>
<body>
<!-- content with classes: rtf-table, rtf-align-*, rtf-valign-*, rtf-u -->
</body>
</html>
```

### Classes Currently in Use

| Class | Purpose | Location |
|-------|---------|----------|
| `rtf-align-left` | Text alignment | paragraph.rs |
| `rtf-align-center` | Text alignment | paragraph.rs |
| `rtf-align-right` | Text alignment | paragraph.rs |
| `rtf-align-justify` | Text alignment | paragraph.rs |
| `rtf-u` | Underline styling | paragraph.rs |
| `rtf-table` | Table border collapse | table.rs |
| `rtf-valign-top` | Cell vertical alignment | table.rs |
| `rtf-valign-middle` | Cell vertical alignment | table.rs |
| `rtf-valign-bottom` | Cell vertical alignment | table.rs |
| `rtf-list-mixed` | Mixed list marker | list.rs |

---

## Implementation Workstreams

### Workstream A: Stylesheet v2

#### Objective
Introduce tokenized CSS with custom properties for improved visual quality and maintainability.

#### CSS Token Structure

Define CSS custom properties in a deterministic order:

```css
:root {
  /* Typography */
  --rtf-font-body: system-ui, -apple-system, "Segoe UI", Roboto, "Helvetica Neue", Arial, sans-serif;
  --rtf-font-mono: "SF Mono", Menlo, Monaco, Consolas, "Liberation Mono", "Courier New", monospace;
  
  /* Font sizes */
  --rtf-text-sm: 0.875rem;
  --rtf-text-base: 1rem;
  --rtf-text-lg: 1.125rem;
  --rtf-text-xl: 1.25rem;
  
  /* Line heights */
  --rtf-leading-normal: 1.5;
  --rtf-leading-relaxed: 1.625;
  
  /* Colors */
  --rtf-color-text: #1f2937;
  --rtf-color-text-muted: #6b7280;
  --rtf-color-background: #ffffff;
  --rtf-color-border: #e5e7eb;
  --rtf-color-link: #2563eb;
  --rtf-color-link-hover: #1d4ed8;
  
  /* Table colors */
  --rtf-color-table-header-bg: #f9fafb;
  --rtf-color-table-stripe-bg: #f9fafb;
  
  /* Spacing */
  --rtf-space-1: 0.25rem;
  --rtf-space-2: 0.5rem;
  --rtf-space-3: 0.75rem;
  --rtf-space-4: 1rem;
  --rtf-space-6: 1.5rem;
  --rtf-space-8: 2rem;
  
  /* Table spacing */
  --rtf-table-cell-padding: var(--rtf-space-2) var(--rtf-space-3);
  --rtf-table-border-width: 1px;
}
```

#### Proposed CSS Styles

```css
/* Document wrapper */
.rtf-doc {
  font-family: var(--rtf-font-body);
  font-size: var(--rtf-text-base);
  line-height: var(--rtf-leading-normal);
  color: var(--rtf-color-text);
  background-color: var(--rtf-color-background);
  max-width: 8.5in;
  margin: 0 auto;
  padding: var(--rtf-space-6);
}

/* Content area */
.rtf-content {
  /* Container for all content blocks */
}

/* Paragraphs */
.rtf-p, p {
  margin: 0 0 var(--rtf-space-4) 0;
}

/* Alignment classes - preserve existing */
.rtf-align-left { text-align: left; }
.rtf-align-center { text-align: center; }
.rtf-align-right { text-align: right; }
.rtf-align-justify { text-align: justify; }

/* Inline formatting */
.rtf-u { text-decoration: underline; }

strong { font-weight: 600; }
em { font-style: italic; }

/* Links */
.rtf-link {
  color: var(--rtf-color-link);
  text-decoration: underline;
}
.rtf-link:hover {
  color: var(--rtf-color-link-hover);
}

/* Lists */
ul, ol {
  margin: 0 0 var(--rtf-space-4) 0;
  padding-left: var(--rtf-space-6);
}

li {
  margin-bottom: var(--rtf-space-1);
}

li > p {
  margin-bottom: 0;
}

.rtf-list-mixed {
  /* Mixed list styling if needed */
}

/* Tables */
.rtf-table {
  border-collapse: collapse;
  width: 100%;
  margin: 0 0 var(--rtf-space-4) 0;
  border: var(--rtf-table-border-width) solid var(--rtf-color-border);
}

.rtf-table td {
  padding: var(--rtf-table-cell-padding);
  border: var(--rtf-table-border-width) solid var(--rtf-color-border);
  vertical-align: top;
}

.rtf-table td p {
  margin: 0;
}

/* Vertical alignment classes - preserve existing */
.rtf-valign-top { vertical-align: top; }
.rtf-valign-middle { vertical-align: middle; }
.rtf-valign-bottom { vertical-align: bottom; }
```

#### Files to Modify

| File | Changes |
|------|---------|
| `crates/rtfkit-html/src/style.rs` | Replace `default_stylesheet()` with new tokenized CSS |
| `crates/rtfkit-html/src/writer.rs` | Add document wrapper class `rtf-doc` |

#### Implementation Steps

1. **Create new CSS constant** in [`style.rs`](crates/rtfkit-html/src/style.rs:79):
   - Define CSS custom properties in `:root`
   - Add `.rtf-doc` wrapper styles
   - Add enhanced table styles
   - Add list styles
   - Preserve existing alignment and valign classes

2. **Add document wrapper class** in [`writer.rs`](crates/rtfkit-html/src/writer.rs:76):
   - Add `class="rtf-doc"` to `<body>` or wrap content in `<div class="rtf-doc">`
   - Consider adding `<div class="rtf-content">` around block content

3. **Update tests** to verify:
   - CSS is deterministic (same output every time)
   - Token ordering is stable
   - No regression in existing golden tests

---

### Workstream B: Print Profile

#### Objective
Add print-friendly CSS rules for graceful degradation when printing or generating PDFs.

#### Print CSS Rules

```css
@media print {
  /* Page setup */
  @page {
    margin: 0.75in 1in;
    size: letter;
  }
  
  /* Document adjustments */
  .rtf-doc {
    max-width: none;
    margin: 0;
    padding: 0;
  }
  
  /* Avoid breaking inside table rows */
  .rtf-table tr {
    break-inside: avoid;
  }
  
  /* Keep paragraphs together where possible */
  p {
    orphans: 3;
    widows: 3;
  }
  
  /* Links in print */
  .rtf-link::after {
    content: " (" attr(href) ")";
    font-size: var(--rtf-text-sm);
    color: var(--rtf-color-text-muted);
  }
  
  /* Hide non-essential elements */
  /* (none currently, but reserved for future use) */
}
```

#### Files to Modify

| File | Changes |
|------|---------|
| `crates/rtfkit-html/src/style.rs` | Add `@media print` block to `default_stylesheet()` |

#### Implementation Steps

1. **Add print media query** to the CSS constant
2. **Test with realworld fixtures**:
   - `fixtures/realworld/annual_report_10p.rtf`
   - `fixtures/realworld/policy_doc_15p.rtf`
   - `fixtures/realworld/technical_spec_12p.rtf`
3. **Manual verification** in browser print preview

---

### Workstream C: CSS Mode/Overrides

#### Objective
Extend `HtmlWriterOptions` with CSS mode selection and custom CSS file support.

#### New Types

```rust
/// CSS emission mode for HTML output.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CssMode {
    /// Embed the built-in polished stylesheet (default).
    #[default]
    Default,
    /// Emit no built-in CSS (for external styling).
    None,
}

/// Configuration options for HTML output.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HtmlWriterOptions {
    /// Whether to emit the full HTML document wrapper.
    pub emit_document_wrapper: bool,
    
    /// CSS emission mode.
    pub css_mode: CssMode,
    
    /// Optional path to a local CSS file to append after built-in CSS.
    /// Only used when css_mode is Default.
    pub custom_css_path: Option<PathBuf>,
}
```

#### Error Handling

Add new error variant to [`HtmlWriterError`](crates/rtfkit-html/src/error.rs:17):

```rust
pub enum HtmlWriterError {
    #[error("HTML generation error: {0}")]
    Generation(String),
    
    #[error("Failed to read custom CSS file: {0}")]
    CustomCssRead(String),
}
```

#### Files to Modify

| File | Changes |
|------|---------|
| `crates/rtfkit-html/src/options.rs` | Add `CssMode` enum, update `HtmlWriterOptions` |
| `crates/rtfkit-html/src/error.rs` | Add `CustomCssRead` error variant |
| `crates/rtfkit-html/src/writer.rs` | Update `emit_document_start()` to handle modes |
| `crates/rtfkit-html/src/lib.rs` | Re-export `CssMode` |

#### CLI Changes

Add new flags to [`main.rs`](crates/rtfkit-cli/src/main.rs):

```rust
/// CSS mode for HTML output
#[arg(long, default_value = "default", value_parser = ["default", "none"])]
html_css: String,

/// Path to custom CSS file to append (optional)
#[arg(long, value_name = "FILE")]
html_css_file: Option<PathBuf>,
```

Update [`handle_html_output()`](crates/rtfkit-cli/src/main.rs:248):

```rust
fn handle_html_output(
    document: &Document,
    output_path: &Path,
    force: bool,
    verbose: bool,
    strict: bool,
    css_mode: CssMode,
    custom_css_path: Option<&Path>,
) -> Result<ExitCode> {
    // Build options with CSS mode
    let options = HtmlWriterOptions {
        emit_document_wrapper: true,
        css_mode,
        custom_css_path: custom_css_path.map(|p| p.to_path_buf()),
    };
    
    // Generate HTML
    let output = document_to_html_with_warnings(document, &options)?;
    // ...
}
```

#### Implementation Steps

1. **Add `CssMode` enum** to [`options.rs`](crates/rtfkit-html/src/options.rs)
2. **Update `HtmlWriterOptions`**:
   - Replace `include_default_css: bool` with `css_mode: CssMode`
   - Add `custom_css_path: Option<PathBuf>`
3. **Update `Default` implementation**:
   - `css_mode: CssMode::Default`
   - `custom_css_path: None`
4. **Update writer** to:
   - Check `css_mode` instead of `include_default_css`
   - Read and append custom CSS file if provided
5. **Add CLI flags**:
   - `--html-css <default|none>`
   - `--html-css-file <FILE>`
6. **Add validation**:
   - `--html-css-file` only valid with `--to html`
   - File must exist and be readable (exit code 3 on failure)
   - Consider adding CSS file size limit (reuse limits pattern)

---

### Workstream D: Documentation

#### Objective
Update documentation to reflect new CSS capabilities.

#### Files to Create/Modify

| File | Action | Content |
|------|--------|---------|
| `docs/reference/html-styling.md` | Create | CSS class reference, token documentation, customization guide |
| `README.md` | Modify | Update HTML examples with new CLI flags |
| `CHANGELOG.md` | Modify | Document new features |

#### Documentation Content

**`docs/reference/html-styling.md`** should include:
1. Overview of CSS architecture
2. CSS custom properties reference
3. Class surface documentation
4. Print styling behavior
5. Custom CSS override instructions
6. PDF preparation guidance

---

## Test Strategy

### Unit Tests (`crates/rtfkit-html`)

| Test | Description |
|------|-------------|
| `test_css_mode_default` | Verify `CssMode::Default` embeds CSS |
| `test_css_mode_none` | Verify `CssMode::None` produces no CSS |
| `test_css_deterministic` | Verify CSS output is identical across calls |
| `test_custom_css_append` | Verify custom CSS is appended after built-in |
| `test_custom_css_missing_file` | Verify error for missing custom CSS file |

### Integration Tests (`crates/rtfkit-cli`)

| Test | Description |
|------|-------------|
| `test_html_css_default_flag` | `--html-css default` succeeds |
| `test_html_css_none_flag` | `--html-css none` produces no CSS |
| `test_html_css_file_valid` | `--html-css-file` with valid file succeeds |
| `test_html_css_file_missing` | Missing file returns exit code 3 |
| `test_html_css_file_unreadable` | Unreadable file returns exit code 3 |
| `test_html_flags_rejected_for_docx` | HTML-only flags rejected for non-HTML output |

### Snapshot Tests

| Test | Description |
|------|-------------|
| Update existing golden HTML tests | Reflect new CSS structure |
| Add CSS snapshot test | Isolate CSS content for easier review |

### Manual Verification

For realworld fixtures:
1. Open in Chromium-based browser
2. Open in Firefox/WebKit
3. Verify print preview
4. Check table rendering
5. Verify link visibility

---

## Migration Notes

### Backward Compatibility

The change from `include_default_css: bool` to `css_mode: CssMode` is a breaking change for users of the library API. However:

1. The CLI default behavior remains the same (CSS embedded)
2. `CssMode::Default` is the default, matching current behavior
3. Users constructing `HtmlWriterOptions` directly will need to update

### Deprecation Path

Consider providing a migration helper:

```rust
impl HtmlWriterOptions {
    /// Compatibility constructor for existing code.
    #[deprecated(since = "0.x.0", note = "Use struct literal with css_mode instead")]
    pub fn new(include_css: bool) -> Self {
        Self {
            emit_document_wrapper: true,
            css_mode: if include_css { CssMode::Default } else { CssMode::None },
            custom_css_path: None,
        }
    }
}
```

---

## File Change Summary

### New Files

| File | Purpose |
|------|---------|
| `docs/reference/html-styling.md` | CSS documentation |

### Modified Files

| File | Changes |
|------|---------|
| `crates/rtfkit-html/src/options.rs` | Add `CssMode`, update `HtmlWriterOptions` |
| `crates/rtfkit-html/src/style.rs` | Replace `default_stylesheet()` with tokenized CSS |
| `crates/rtfkit-html/src/writer.rs` | Handle CSS modes, custom CSS, add wrapper classes |
| `crates/rtfkit-html/src/error.rs` | Add `CustomCssRead` error |
| `crates/rtfkit-html/src/lib.rs` | Re-export `CssMode` |
| `crates/rtfkit-cli/src/main.rs` | Add `--html-css` and `--html-css-file` flags |
| `crates/rtfkit-cli/tests/golden_tests.rs` | Update for new CSS structure |
| `README.md` | Update examples |
| `CHANGELOG.md` | Document changes |

### Golden Files to Update

All files in `golden_html/` will need updating due to CSS changes:
- Run `UPDATE_GOLDEN=1 cargo test -p rtfkit-cli` after implementation

---

## Implementation Order

1. **Workstream A** (Stylesheet v2) - Foundation for visual improvements
2. **Workstream C** (CSS Mode) - Enable mode selection
3. **Workstream B** (Print Profile) - Add print styles to the new CSS
4. **Workstream D** (Documentation) - Document completed features
5. **Update golden tests** - Capture new output

---

## Acceptance Criteria Checklist

From spec section 11:

- [ ] HTML output looks materially better on representative fixtures with no parser changes
- [ ] Built-in CSS is deterministic and covered by tests
- [ ] `--html-css` mode selection behaves as documented
- [ ] Optional local CSS override is supported with clear IO failures
- [ ] Exit codes, strict mode, warnings, and security contracts remain intact
- [ ] Documentation is updated with styling behavior and examples

---

## Risk Mitigation

| Risk | Mitigation |
|------|------------|
| CSS churn breaks snapshot stability | Isolate stylesheet snapshots, gate changes in PR review |
| Style complexity grows | Enforce non-goals, prioritize readability over fidelity |
| Override flags complicate CLI UX | Keep default simple, reserve advanced flags for explicit use |
| Backend variance across browsers | Document expected variability, keep CSS conservative |
