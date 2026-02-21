#!/bin/bash
#
# Smoke Test Script for rtfkit (Unix)
# Verifies that the binary runs correctly and basic conversion works.
#
# Usage: ./smoke_test.sh <path-to-rtfkit-binary>
#
# Exit codes:
#   0 - All tests passed
#   1 - Test failure
#   2 - Missing arguments or invalid state

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Test counters
TESTS_PASSED=0
TESTS_FAILED=0

# Helper functions
log_info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

log_test() {
    echo -e "\n${GREEN}=== TEST: $1 ===${NC}"
}

pass() {
    log_info "✅ PASSED: $1"
    TESTS_PASSED=$((TESTS_PASSED + 1))
}

fail() {
    log_error "❌ FAILED: $1"
    TESTS_FAILED=$((TESTS_FAILED + 1))
}

cleanup() {
    if [[ -n "${WORK_DIR:-}" && -d "$WORK_DIR" ]]; then
        rm -rf "$WORK_DIR"
    fi
}

trap cleanup EXIT

# Check arguments
if [[ $# -lt 1 ]]; then
    echo "Usage: $0 <path-to-rtfkit-binary>"
    echo "Example: $0 ./target/release/rtfkit"
    exit 2
fi

RTFKIT="$1"

# Verify binary exists
if [[ ! -x "$RTFKIT" ]]; then
    log_error "Binary not found or not executable: $RTFKIT"
    exit 2
fi

# Create temporary working directory
WORK_DIR=$(mktemp -d)
log_info "Working directory: $WORK_DIR"

# ============================================
# Test 1: Binary runs and shows version
# ============================================
log_test "Binary version check"

VERSION_OUTPUT=$("$RTFKIT" --version 2>&1) || {
    fail "Binary failed to execute with --version"
    VERSION_OUTPUT=""
}

if [[ -n "$VERSION_OUTPUT" ]]; then
    log_info "Version output: $VERSION_OUTPUT"
    pass "Binary executes and shows version"
else
    fail "No version output received"
fi

# ============================================
# Test 2: Binary shows help
# ============================================
log_test "Help output check"

HELP_OUTPUT=$("$RTFKIT" --help 2>&1) || {
    fail "Binary failed to execute with --help"
    HELP_OUTPUT=""
}

if [[ "$HELP_OUTPUT" == *"rtfkit"* && "$HELP_OUTPUT" == *"convert"* ]]; then
    log_info "Help output contains expected content"
    pass "Help output is valid"
else
    fail "Help output missing expected content"
fi

# ============================================
# Test 3: Convert simple RTF to DOCX
# ============================================
log_test "Basic RTF to DOCX conversion"

# Create a simple RTF file
SIMPLE_RTF="$WORK_DIR/simple.rtf"
cat > "$SIMPLE_RTF" << 'EOF'
{\rtf1\ansi\deff0
{\fonttbl{\f0 Times New Roman;}}
\viewkind4\uc1\pard\f0\fs24
Hello, World!\par
}
EOF

OUTPUT_DOCX="$WORK_DIR/output.docx"

if "$RTFKIT" convert "$SIMPLE_RTF" --output "$OUTPUT_DOCX" 2>&1; then
    if [[ -f "$OUTPUT_DOCX" ]]; then
        # Verify it's a valid ZIP (DOCX is a ZIP file)
        if unzip -t "$OUTPUT_DOCX" >/dev/null 2>&1; then
            log_info "DOCX file is a valid ZIP archive"
            
            # Check for required DOCX components
            if unzip -l "$OUTPUT_DOCX" 2>/dev/null | grep -q "word/document.xml"; then
                pass "RTF to DOCX conversion produces valid DOCX"
            else
                fail "DOCX missing word/document.xml"
            fi
        else
            fail "Output is not a valid ZIP/DOCX file"
        fi
    else
        fail "Output DOCX file was not created"
    fi
else
    fail "Conversion command failed"
fi

# ============================================
# Test 4: Convert RTF to JSON report
# ============================================
log_test "RTF to JSON report conversion"

JSON_OUTPUT=$("$RTFKIT" convert "$SIMPLE_RTF" --format json 2>&1) || {
    fail "JSON report generation failed"
    JSON_OUTPUT=""
}

if [[ -n "$JSON_OUTPUT" ]]; then
    # Verify it's valid JSON
    if echo "$JSON_OUTPUT" | jq empty 2>/dev/null; then
        # Check for expected fields
        if echo "$JSON_OUTPUT" | jq -e '.stats.paragraph_count' >/dev/null 2>&1; then
            log_info "JSON report contains stats"
            pass "JSON report generation works"
        else
            fail "JSON report missing stats field"
        fi
    else
        fail "Output is not valid JSON"
    fi
else
    fail "No JSON output received"
fi

# ============================================
# Test 5: Convert RTF to text report
# ============================================
log_test "RTF to text report conversion"

TEXT_OUTPUT=$("$RTFKIT" convert "$SIMPLE_RTF" --format text 2>&1) || {
    fail "Text report generation failed"
    TEXT_OUTPUT=""
}

if [[ -n "$TEXT_OUTPUT" && "$TEXT_OUTPUT" == *"Conversion Report"* ]]; then
    log_info "Text report contains expected header"
    pass "Text report generation works"
else
    fail "Text report missing expected content"
fi

# ============================================
# Test 6: Emit IR to JSON file
# ============================================
log_test "IR emission to JSON file"

IR_FILE="$WORK_DIR/ir.json"

if "$RTFKIT" convert "$SIMPLE_RTF" --emit-ir "$IR_FILE" 2>&1; then
    if [[ -f "$IR_FILE" ]]; then
        if jq empty "$IR_FILE" 2>/dev/null; then
            if jq -e '.blocks' "$IR_FILE" >/dev/null 2>&1; then
                log_info "IR file contains blocks array"
                pass "IR emission works"
            else
                fail "IR file missing blocks field"
            fi
        else
            fail "IR file is not valid JSON"
        fi
    else
        fail "IR file was not created"
    fi
else
    fail "IR emission command failed"
fi

# ============================================
# Test 7: Handle non-existent file gracefully
# ============================================
log_test "Error handling for non-existent file"

ERROR_OUTPUT=$("$RTFKIT" convert "$WORK_DIR/nonexistent.rtf" 2>&1) && EXIT_CODE=0 || EXIT_CODE=$?

if [[ $EXIT_CODE -ne 0 ]]; then
    log_info "Non-zero exit code: $EXIT_CODE"
    if [[ "$ERROR_OUTPUT" == *"Failed to read"* || "$ERROR_OUTPUT" == *"Error"* ]]; then
        pass "Error handling for missing file works"
    else
        fail "Error message not informative"
    fi
else
    fail "Should have failed for non-existent file"
fi

# ============================================
# Test 8: Strict mode with clean document
# ============================================
log_test "Strict mode with clean document"

STRICT_OUTPUT=$("$RTFKIT" convert "$SIMPLE_RTF" --strict 2>&1) || {
    fail "Strict mode failed on clean document"
    STRICT_OUTPUT=""
}

if [[ -n "$STRICT_OUTPUT" ]]; then
    pass "Strict mode works with clean document"
else
    fail "No output in strict mode"
fi

# ============================================
# Test 9: Force flag overwrites existing file
# ============================================
log_test "Force flag overwrites existing file"

EXISTING_DOCX="$WORK_DIR/existing.docx"
echo "dummy" > "$EXISTING_DOCX"

# First try without force - should fail
NO_FORCE_OUTPUT=$("$RTFKIT" convert "$SIMPLE_RTF" --output "$EXISTING_DOCX" 2>&1) && EXIT_CODE=0 || EXIT_CODE=$?

if [[ $EXIT_CODE -ne 0 && "$NO_FORCE_OUTPUT" == *"already exists"* ]]; then
    log_info "Correctly refuses to overwrite without --force"
    
    # Now try with force - should succeed
    FORCE_OUTPUT=$("$RTFKIT" convert "$SIMPLE_RTF" --output "$EXISTING_DOCX" --force 2>&1) || {
        fail "Force flag conversion failed"
    }
    
    if [[ -f "$EXISTING_DOCX" ]] && unzip -t "$EXISTING_DOCX" >/dev/null 2>&1; then
        pass "Force flag overwrites existing file"
    else
        fail "Force flag did not create valid output"
    fi
else
    fail "Should have refused to overwrite without --force"
fi

# ============================================
# Test 10: Complex RTF with tables
# ============================================
log_test "Complex RTF with table conversion"

TABLE_RTF="$WORK_DIR/table.rtf"
cat > "$TABLE_RTF" << 'EOF'
{\rtf1\ansi\deff0
{\fonttbl{\f0 Arial;}}
\trowd\cellx1000\cellx2000
\intbl Cell 1\cell
\intbl Cell 2\cell
\row
\trowd\cellx1000\cellx2000
\intbl Cell 3\cell
\intbl Cell 4\cell
\row
}
EOF

TABLE_DOCX="$WORK_DIR/table.docx"

if "$RTFKIT" convert "$TABLE_RTF" --output "$TABLE_DOCX" 2>&1; then
    if [[ -f "$TABLE_DOCX" ]] && unzip -t "$TABLE_DOCX" >/dev/null 2>&1; then
        pass "Table RTF conversion works"
    else
        fail "Table DOCX output invalid"
    fi
else
    fail "Table RTF conversion failed"
fi

# ============================================
# Summary
# ============================================
echo ""
echo "============================================"
echo "           SMOKE TEST SUMMARY"
echo "============================================"
echo ""
echo -e "Tests Passed: ${GREEN}$TESTS_PASSED${NC}"
echo -e "Tests Failed: ${RED}$TESTS_FAILED${NC}"
echo ""

if [[ $TESTS_FAILED -eq 0 ]]; then
    echo -e "${GREEN}✅ All smoke tests passed!${NC}"
    exit 0
else
    echo -e "${RED}❌ Some smoke tests failed!${NC}"
    exit 1
fi
