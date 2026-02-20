# rtfkit Next Steps Plan (Phase 5: Images / `\pict`)

This phase adds first-class image handling for common embedded image payloads.

## 1. Primary objective

Support a practical subset of RTF `\pict` content (at minimum PNG/JPEG where feasible) without compromising parser safety.

## 2. In scope

- Parse and decode `\pict` payloads for supported formats.
- Represent images in IR with metadata required by writers.
- Write supported images into DOCX package with correct relationships.
- Provide graceful fallback for unsupported/corrupt payloads.

## 3. Out of scope

- Full `\pict` format coverage and all legacy image encodings.
- Advanced positioning/anchoring fidelity in Word.

## 4. Implementation direction

- Extend parser/interpreter to extract image payload + basic dimensions when available.
- Add IR image block model.
- DOCX writer emits media parts and references from document XML.
- Fallback for unsupported payload:
  - warning
  - optional placeholder text
  - strict-mode failure when semantics are dropped

## 5. Testing and fixtures

- At least:
  - one PNG fixture (success)
  - one JPEG fixture (success)
  - one unsupported/corrupt fixture (fallback)
- Validate that DOCX package contains expected media entries and references.

## 6. Acceptance criteria

- Supported image fixtures render visible images in Word/LibreOffice.
- Unsupported cases degrade safely and predictably with warnings.
- No parser instability from untrusted/big image payloads.

