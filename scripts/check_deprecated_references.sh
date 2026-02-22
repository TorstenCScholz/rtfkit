#!/bin/bash
#
# check_deprecated_references.sh
# Scans active code/docs for references to removed legacy PDF artifacts
#
# Exit codes:
#   0 - No references found
#   1 - References to deprecated files found (error)
#
# This script should be run before commits to ensure no active code/docs
# still reference removed legacy paths or flags.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

echo "Checking for references to removed legacy PDF artifacts..."

# Check for references to rtfkit-pdf crate in Cargo.toml files
echo "Checking Cargo.toml files for rtfkit-pdf references..."
if grep -r "rtfkit-pdf" "$PROJECT_ROOT/Cargo.toml" "$PROJECT_ROOT"/crates/*/Cargo.toml 2>/dev/null; then
    echo "ERROR: Found references to rtfkit-pdf in Cargo.toml files"
    echo "The deprecated crate should not be a dependency of active crates."
    exit 1
fi

# Check for imports of rtfkit_pdf in Rust source files
echo "Checking Rust source files for rtfkit_pdf imports..."
if find "$PROJECT_ROOT/crates" -name "*.rs" -exec grep -l "rtfkit_pdf" {} \; 2>/dev/null | head -1 | grep -q .; then
    echo "ERROR: Found imports of rtfkit_pdf in active Rust source files"
    exit 1
fi

# Check for references to superseded planning docs in active documentation
echo "Checking active docs for references to superseded PDF planning docs..."
DEPRECATED_DOCS=(
    "0003-pdf-backend-selection"
    "PHASE_PDF"
)

for pattern in "${DEPRECATED_DOCS[@]}"; do
    # Search only active docs
    if find "$PROJECT_ROOT/docs" -name "*.md" -exec grep -l "$pattern" {} \; 2>/dev/null | head -1 | grep -q .; then
        echo "ERROR: Found references to deprecated doc '$pattern' in active documentation"
        exit 1
    fi
done

# Check for any path references to deprecated/ directory in active code/docs
echo "Checking for path references to deprecated/ directory..."
# Check crates/ for references
if find "$PROJECT_ROOT/crates" -type f \( -name "*.rs" -o -name "*.toml" \) \
    -exec grep -l "deprecated/" {} \; 2>/dev/null | head -1 | grep -q .; then
    echo "ERROR: Found references to deprecated/ directory in active code"
    exit 1
fi

# Check top-level active docs for references
if grep -l "deprecated/" "$PROJECT_ROOT/README.md" "$PROJECT_ROOT/CONTRIBUTING.md" "$PROJECT_ROOT/CHANGELOG.md" 2>/dev/null | head -1 | grep -q .; then
    echo "ERROR: Found references to deprecated/ directory in top-level docs"
    exit 1
fi

# Check docs/ markdown files for references
if find "$PROJECT_ROOT/docs" -type f -name "*.md" \
    -exec grep -l "deprecated/" {} \; 2>/dev/null | head -1 | grep -q .; then
    echo "ERROR: Found references to deprecated/ directory in active docs"
    exit 1
fi

echo "✓ No references to removed legacy PDF artifacts found"
exit 0
