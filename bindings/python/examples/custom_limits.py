"""Parse with custom resource limits."""

import rtfkit

# Restrictive limits
limits = rtfkit.ParserLimits(
    max_input_bytes=1024,
    max_group_depth=32,
    max_warning_count=10,
)

rtf = r"{\rtf1 Short document}"
try:
    result = rtfkit.parse_with_limits(rtf, limits)
    print(f"OK: {len(result.document)} blocks")
except rtfkit.ParseError as e:
    print(f"Rejected: {e}")

# Unlimited (for trusted input)
unlimited = rtfkit.ParserLimits.unlimited()
print(f"Unlimited max_input_bytes: {unlimited.max_input_bytes}")
