#!/bin/bash
#
# CI Guardrails: Check for hardcoded style constants outside adapter modules
#
# This script enforces the Cross-Format Style Tokens Framework policy that
# style constants (colors, font sizes, spacing values) should only be defined
# in designated adapter modules:
#
#   - crates/rtfkit-html/src/style.rs (HTML adapter)
#   - crates/rtfkit-style-tokens/src/serialize.rs (serialization module)
#   - crates/rtfkit-style-tokens/src/builtins.rs (built-in profiles)
#   - crates/rtfkit-style-tokens/src/profile.rs (profile definitions)
#
# Exit codes:
#   0 - No violations found
#   1 - Violations found (prints error details)

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Track violations
VIOLATIONS=0

# Adapter modules where style constants are allowed
ADAPTER_MODULES=(
    "crates/rtfkit-html/src/style.rs"
    "crates/rtfkit-style-tokens/src/serialize.rs"
    "crates/rtfkit-style-tokens/src/builtins.rs"
    "crates/rtfkit-style-tokens/src/profile.rs"
)

# Function to check if a file is an adapter module
is_adapter_module() {
    local file="$1"
    for adapter in "${ADAPTER_MODULES[@]}"; do
        if [[ "$file" == "$adapter" ]]; then
            return 0
        fi
    done
    return 1
}

# Function to check if file is a test file (allowed to have some hardcoded values)
is_test_file() {
    local file="$1"
    if [[ "$file" == *"tests/"* ]] || [[ "$file" == *"_test.rs" ]] || [[ "$file" == *"test_"* ]]; then
        return 0
    fi
    return 1
}

# Function to check if file is documentation
is_documentation() {
    local file="$1"
    if [[ "$file" == *.md ]] || [[ "$file" == *"/docs/"* ]]; then
        return 0
    fi
    return 1
}

echo "Checking for hardcoded style constants outside adapter modules..."
echo ""

# Find all Rust source files in crates directory
while IFS= read -r -d '' file; do
    # Skip adapter modules
    if is_adapter_module "$file"; then
        continue
    fi
    
    # Skip test files (they often have test fixtures)
    if is_test_file "$file"; then
        continue
    fi
    
    # Skip documentation
    if is_documentation "$file"; then
        continue
    fi
    
    # Check for hardcoded hex colors (like #ffffff, #000000, #1A1A1A)
    # Pattern matches: "#XXXXXX" where X is hex digit
    # Exclude common non-style uses like #[derive(...)] or #[test]
    color_matches=$(grep -nE '"#[0-9A-Fa-f]{6}"' "$file" 2>/dev/null || true)
    if [[ -n "$color_matches" ]]; then
        # Filter out false positives (derive attributes, test attributes)
        color_matches=$(echo "$color_matches" | grep -v '#\[derive' | grep -v '#\[test' | grep -v '#\[cfg' || true)
        if [[ -n "$color_matches" ]]; then
            echo -e "${RED}VIOLATION: Hardcoded hex color in $file${NC}"
            echo "$color_matches"
            echo ""
            ((VIOLATIONS++))
        fi
    fi
    
    # Check for hardcoded font sizes with pt unit
    # Pattern matches: "XXpt" or 'XXpt' where XX is a number
    # This catches things like "12pt", "14pt" in string literals
    size_matches=$(grep -nE '"[0-9]+pt"' "$file" 2>/dev/null || true)
    if [[ -n "$size_matches" ]]; then
        # Filter out comments
        size_matches=$(echo "$size_matches" | grep -v '^\s*//' || true)
        if [[ -n "$size_matches" ]]; then
            echo -e "${YELLOW}WARNING: Hardcoded font size (pt) in $file${NC}"
            echo "$size_matches"
            echo ""
            # This is a warning, not an error, for now
        fi
    fi
    
    # Check for hardcoded font sizes with px unit
    size_px_matches=$(grep -nE '"[0-9]+px"' "$file" 2>/dev/null || true)
    if [[ -n "$size_px_matches" ]]; then
        size_px_matches=$(echo "$size_px_matches" | grep -v '^\s*//' || true)
        if [[ -n "$size_px_matches" ]]; then
            echo -e "${YELLOW}WARNING: Hardcoded font size (px) in $file${NC}"
            echo "$size_px_matches"
            echo ""
        fi
    fi
    
    # Check for hardcoded spacing values in format strings
    # Pattern: format!(... "{}pt" ...) outside adapter modules
    spacing_matches=$(grep -nE 'format!.*"[0-9]+pt"' "$file" 2>/dev/null || true)
    if [[ -n "$spacing_matches" ]]; then
        spacing_matches=$(echo "$spacing_matches" | grep -v '^\s*//' || true)
        if [[ -n "$spacing_matches" ]]; then
            echo -e "${YELLOW}WARNING: Hardcoded spacing value in format string in $file${NC}"
            echo "$spacing_matches"
            echo ""
        fi
    fi

done < <(find crates -name "*.rs" -print0 2>/dev/null)

echo ""
echo "========================================"

if [[ $VIOLATIONS -gt 0 ]]; then
    echo -e "${RED}FAILED: $VIOLATIONS violation(s) found${NC}"
    echo ""
    echo "Style constants should be defined in adapter modules:"
    echo "  - crates/rtfkit-html/src/style.rs"
    echo "  - crates/rtfkit-style-tokens/src/serialize.rs"
    echo "  - crates/rtfkit-style-tokens/src/builtins.rs"
    echo "  - crates/rtfkit-style-tokens/src/profile.rs"
    echo ""
    echo "See: plans/cross-format-style-tokens-spec.md (Section 11 - CI Guardrails)"
    exit 1
else
    echo -e "${GREEN}PASSED: No hardcoded style constant violations found${NC}"
    exit 0
fi
