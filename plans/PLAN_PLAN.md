Best next feature bundles (time-coupled) from your current roadmap:

Images first: PLAN_IMAGES.md + PLAN_IMAGES_PICT.md
Rationale: biggest real-world fidelity gap; touches parser/IR/all writers once. Doing it in one pass avoids repeated pipeline churn.

Linking/navigation bundle: PLAN_FIELD_SUPPORT_EXPANSION.md + PLAN_INTERNAL_LINKS_AND_BOOKMARKS.md
Rationale: same field parsing surface (\field, \fldinst, \fldrslt) and same strict-mode degradation rules. Implement together to avoid reworking field state machine twice.

Document-structure bundle: PLAN_HEADERS_FOOTERS.md + PLAN_FOOTNOTES_ENDNOTES.md
Rationale: both are destination-scope content outside normal body flow; same kind of IR expansion and writer plumbing.

Table fidelity bundle: PLAN_TABLE_LAYOUT_FIDELITY.md + PLAN_TABLE_BORDERS_PARITY.md + PLAN_NESTED_TABLES_AND_COMPLEX_CELLS.md
Rationale: all are table model/schema changes and writer mapping. Doing separately creates repeated breaking changes in table IR and normalization logic.

Style system bundle: PLAN_THEME_AND_STYLESHEET_MAPPING.md + PLAN_DOCX_STYLE_PROFILES.md
Rationale: token/style precedence should be defined once across formats; DOCX profile work benefits directly from theme/stylesheet mapping decisions.

Text-effects follow-up: PLAN_ADVANCED_TEXT_EFFECTS.md
Rationale: should come after style precedence rules are settled, so effects/reset semantics are consistent across outputs.

PDF custom fonts last: PLAN_PDF_CUSTOM_FONT_SUPPORT.md
Rationale: renderer-specific, relatively isolated, lower cross-pipeline coupling than the above.

If you want a strict execution order with highest ROI/risk balance: 1 → 4 → 2 → 3 → 5 → 6 → 7.