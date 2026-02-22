# HTML Styling Reference

This document describes the HTML styling system in rtfkit, including CSS classes, design tokens, and customization options.

## CSS Output Modes

rtfkit supports two CSS output modes for HTML:

| Mode | Description |
|------|-------------|
| `default` | Embed the built-in polished stylesheet (default) |
| `none` | Omit built-in CSS, emit semantic HTML only |

### CLI Usage

Use the `--html-css` flag to control CSS output:

```bash
# Default: embed built-in CSS
rtfkit convert document.rtf --to html --output document.html

# Explicit default mode
rtfkit convert document.rtf --to html --html-css default --output document.html

# No built-in CSS (semantic HTML only)
rtfkit convert document.rtf --to html --html-css none --output document.html
```

### Custom CSS

Use `--html-css-file` to append custom CSS after the built-in stylesheet:

```bash
rtfkit convert document.rtf --to html --html-css-file custom.css --output document.html
```

Custom CSS is appended after built-in CSS, allowing you to override default styles.

## CSS Classes

The following semantic classes are used in HTML output:

### Document Structure

| Class | Element | Description |
|-------|---------|-------------|
| `rtf-doc` | `<div>` | Document wrapper, sets font and text color |
| `rtf-content` | `<div>` | Content container with max-width and padding |

### Text Elements

| Class | Element | Description |
|-------|---------|-------------|
| `rtf-p` | `<p>` | Paragraph styling |
| `rtf-link` | `<a>` | Link styling with hover state |
| `rtf-u` | `<span>` | Underlined text |

### Alignment

| Class | Description |
|-------|-------------|
| `rtf-align-left` | Left-aligned text |
| `rtf-align-center` | Centered text |
| `rtf-align-right` | Right-aligned text |
| `rtf-align-justify` | Justified text |

### Tables

| Class | Element | Description |
|-------|---------|-------------|
| `rtf-table` | `<table>` | Table styling with borders and row striping |
| `rtf-valign-top` | `<td>`, `<th>` | Top vertical alignment |
| `rtf-valign-middle` | `<td>`, `<th>` | Middle vertical alignment |
| `rtf-valign-bottom` | `<td>`, `<th>` | Bottom vertical alignment |

### Lists

| Class | Element | Description |
|-------|---------|-------------|
| `rtf-list` | `<ul>`, `<ol>` | List container styling |
| `rtf-list-mixed` | `<ul>`, `<ol>` | Mixed list types (additional class) |

## Design Tokens

The built-in stylesheet uses CSS custom properties (design tokens) for consistent styling:

### Typography

```css
--rtf-font-body: system-ui, -apple-system, "Segoe UI", Roboto, "Helvetica Neue", Arial, sans-serif;
--rtf-font-mono: "SF Mono", Menlo, Monaco, Consolas, "Liberation Mono", "Courier New", monospace;
```

### Colors

```css
--rtf-color-text: #1a1a1a;
--rtf-color-text-muted: #5a5a5a;
--rtf-color-background: #ffffff;
--rtf-color-link: #0066cc;
--rtf-color-link-hover: #004499;
--rtf-color-border: #d0d0d0;
--rtf-color-border-light: #e8e8e8;
--rtf-color-table-header-bg: #f5f5f5;
--rtf-color-table-stripe-bg: #fafafa;
```

### Spacing Scale

```css
--rtf-space-xs: 0.25rem;
--rtf-space-sm: 0.5rem;
--rtf-space-md: 1rem;
--rtf-space-lg: 1.5rem;
--rtf-space-xl: 2rem;
```

### Table Sizing

```css
--rtf-table-cell-padding: 0.5rem 0.75rem;
--rtf-table-border-width: 1px;
```

## Overriding Styles

To customize the default appearance, create a CSS file that overrides the design tokens:

```css
/* custom.css */
:root {
  --rtf-color-link: #8b0000;
  --rtf-color-table-header-bg: #e0e0ff;
  --rtf-space-md: 1.25rem;
}

.rtf-doc {
  font-size: 14px;
}
```

Then use it with:

```bash
rtfkit convert document.rtf --to html --html-css-file custom.css --output document.html
```

## Print Styles

The built-in stylesheet includes print-specific styles:

- A4 page size with 2cm margins
- Black text on white background
- Link URLs shown after link text
- Table rows kept together when possible
- Widow/orphan control for paragraphs

## Preparing for PDF

The HTML output is designed to work well with PDF generation tools. For best results:

1. Use the default CSS mode for consistent styling
2. Test print preview in your browser
3. Use print-to-PDF tools that support CSS @media print rules
