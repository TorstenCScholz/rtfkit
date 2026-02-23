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
--rtfkit-font-body: "Libertinus Serif";
--rtfkit-font-heading: "Libertinus Serif";
--rtfkit-font-mono: "DejaVu Sans Mono";
--rtfkit-size-body: 11pt;
--rtfkit-size-h1: 26pt;
```

### Colors

```css
--rtfkit-color-text-primary: #1A1A1A;
--rtfkit-color-text-muted: #5A5A5A;
--rtfkit-color-surface-page: #FFFFFF;
--rtfkit-color-link-default: #2563EB;
--rtfkit-color-link-hover: #1D4ED8;
--rtfkit-color-border-default: #D1D5DB;
--rtfkit-color-surface-table-header: #E6E9ED;
--rtfkit-color-surface-table-stripe: #F4F6F8;
```

### Spacing Scale

```css
--rtfkit-space-xs: 3pt;
--rtfkit-space-sm: 6pt;
--rtfkit-space-md: 10pt;
--rtfkit-space-lg: 18pt;
--rtfkit-space-xl: 28pt;
--rtfkit-paragraph-gap: 12pt;
--rtfkit-list-item-gap: 6pt;
```

### Table Sizing

```css
--rtfkit-table-cell-padding-x: 8pt;
--rtfkit-table-cell-padding-y: 5pt;
--rtfkit-table-border-width: 0.5pt;
--rtfkit-table-stripe-mode: alternate-rows;
--rtfkit-table-stripe-fill: #F4F6F8;
--rtfkit-table-header-font-weight: 600;
```

## Overriding Styles

To customize the default appearance, create a CSS file that overrides the design tokens:

```css
/* custom.css */
:root {
  --rtfkit-color-link-default: #8b0000;
  --rtfkit-color-surface-table-header: #e0e0ff;
  --rtfkit-space-md: 12pt;
}

.rtf-doc {
  font-size: 14px;
}
```

Then use it with:

```bash
rtfkit convert document.rtf --to html --html-css-file custom.css --output document.html
```

## Style Profiles

rtfkit uses **style profiles** to provide consistent visual styling across HTML and PDF output formats. A style profile defines typography, colors, spacing, and layout parameters that are translated to format-specific output (CSS for HTML, Typst preamble for PDF).

### Built-in Profiles

Three built-in profiles are available:

| Profile | Description |
|---------|-------------|
| `classic` | Conservative, neutral styling close to original RTF appearance |
| `report` | Strong visual hierarchy optimized for long-form documents (default) |
| `compact` | Dense styling for enterprise output with reduced whitespace |

### CLI Usage

Use the `--style-profile` flag to select a profile:

```bash
# Use the report profile (default)
rtfkit convert document.rtf --to html --output document.html

# Use the classic profile
rtfkit convert document.rtf --to html --style-profile classic --output document.html

# Use the compact profile
rtfkit convert document.rtf --to html --style-profile compact --output document.html
```

### CSS Variable Generation

When generating HTML output, the style profile is converted to CSS custom properties (variables) that control the visual appearance. For example, the `report` profile generates CSS variables for:

- **Typography**: Font families, sizes, weights for body text and headings
- **Colors**: Text colors, background colors, link colors, border colors
- **Spacing**: Paragraph gaps, list indentation, table cell padding
- **Layout**: Content max-width, page margins

The generated CSS variables follow the naming convention shown in the [Design Tokens](#design-tokens) section. Each profile produces different values for these variables, enabling consistent visual identity across documents.

### Combining with Custom CSS

Style profiles work together with the `--html-css` and `--html-css-file` options:

```bash
# Use a profile and append custom CSS overrides
rtfkit convert document.rtf --to html --style-profile report --html-css-file custom.css --output document.html
```

Custom CSS is appended after the profile-generated CSS, allowing you to override specific variables or add additional styles.

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
