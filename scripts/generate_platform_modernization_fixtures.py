#!/usr/bin/env python3
"""Generate realworld platform-modernization RTF fixtures with deterministic charts.

This script creates:
- platform_modernization_showcase_12p.rtf (strict-pass)
- platform_modernization_warning_probe_10p.rtf (strict-fail due to dropped content)

Charts are generated deterministically from synthetic data and embedded as PNG/JPEG hex.
"""

from __future__ import annotations

import argparse
import json
import math
import random
import tempfile
from dataclasses import dataclass
from pathlib import Path
from typing import Iterable

try:
    import matplotlib
    matplotlib.use("Agg")
    import matplotlib.pyplot as plt
except ModuleNotFoundError as exc:
    raise SystemExit(
        "Missing dependency: matplotlib. Install it in your Python environment "
        "or run inside a virtualenv before generating fixtures."
    ) from exc


DEFAULT_SEED = 20260303
DEFAULT_SHOWCASE_PAGES = 12
DEFAULT_WARNING_PAGES = 10


SECTIONS: list[tuple[str, str]] = [
    ("Executive Summary and Recommendation", "sec_exec_summary"),
    ("Historical Context and Stakeholder Alignment Gaps", "sec_historical_context"),
    ("Status Quo Constraints", "sec_status_quo_constraints"),
    ("FinOps and Operations", "sec_finops_operations"),
    ("Target Platform Vision", "sec_target_platform_vision"),
    ("Dual-Track Strategy", "sec_dual_track_strategy"),
    ("Pilot Proof: Service Operations Console", "sec_pilot_proof"),
    ("Security and Compliance Implications", "sec_security_compliance"),
    ("Implementation Roadmap", "sec_implementation_roadmap"),
    ("Risk Register and Mitigations", "sec_risk_register"),
    ("FAQ for Executives and Engineering", "sec_faq"),
    ("Appendix: Migration Scenarios", "sec_appendix"),
]


@dataclass(frozen=True)
class ChartAsset:
    name: str
    blip_kind: str
    hex_payload: str
    width_twips: int
    height_twips: int
    scalex: int
    scaley: int


def escape_rtf_text(text: str) -> str:
    """Escape plain text for RTF and emit non-ASCII via \\uN? escapes."""
    out: list[str] = []
    for ch in text:
        code = ord(ch)
        if ch == "\\":
            out.append(r"\\")
        elif ch == "{":
            out.append(r"\{")
        elif ch == "}":
            out.append(r"\}")
        elif ch == "\n":
            out.append(r"\line ")
        elif ch == "\t":
            out.append(r"\tab ")
        elif 32 <= code < 127:
            out.append(ch)
        else:
            signed = code if code <= 32767 else code - 65536
            out.append(f"\\u{signed}?")
    return "".join(out)


def embed_pict(
    blip_kind: str,
    hex_payload: str,
    width_twips: int,
    height_twips: int,
    scalex: int,
    scaley: int,
) -> str:
    """Emit an RTF \\pict group with common size/scaling controls."""
    return (
        "{\\pict\\"
        + blip_kind
        + f"\\picwgoal{width_twips}\\pichgoal{height_twips}"
        + f"\\picw{max(1, width_twips // 15)}\\pich{max(1, height_twips // 15)}"
        + f"\\picscalex{scalex}\\picscaley{scaley} "
        + hex_payload
        + "}"
    )


def _finalize_figure(path: Path, fmt: str) -> None:
    plt.tight_layout()
    if fmt == "png":
        plt.savefig(path, format="png", dpi=140, metadata={"Creation Time": None})
    else:
        plt.savefig(path, format=fmt, dpi=140)
    plt.close()


def _chart_ram(path: Path, rng: random.Random) -> None:
    months = [f"M{i}" for i in range(1, 13)]
    requested = [18.0 + 0.25 * i + rng.uniform(-0.3, 0.3) for i in range(12)]
    used = [req * (0.58 + 0.03 * math.sin(i / 2.0)) for i, req in enumerate(requested)]
    p95 = [u * 1.12 for u in used]
    p50 = [u * 0.88 for u in used]

    plt.figure(figsize=(8.6, 4.2))
    plt.plot(months, requested, label="Requested RAM (GiB)", linewidth=2.3, color="#264653")
    plt.plot(months, used, label="Used RAM (GiB)", linewidth=2.1, color="#2A9D8F")
    plt.fill_between(months, p50, p95, alpha=0.2, color="#E76F51", label="Used RAM P50-P95")
    plt.title("Service Tier Memory Profile")
    plt.ylabel("GiB")
    plt.grid(alpha=0.25)
    plt.legend(loc="upper left")
    _finalize_figure(path, "png")


def _chart_cost(path: Path, rng: random.Random) -> None:
    quarters = ["Q1", "Q2", "Q3", "Q4", "Q5", "Q6"]
    infra = [82 + i * 3 + rng.uniform(-2.0, 2.0) for i in range(6)]
    data = [41 + i * 2 + rng.uniform(-1.0, 1.0) for i in range(6)]
    support = [26 + i * 1.3 + rng.uniform(-0.8, 0.8) for i in range(6)]
    optimized_overlay = [
        (infra[i] + data[i] + support[i]) * (0.78 if i >= 3 else 1.0) for i in range(6)
    ]

    plt.figure(figsize=(8.6, 4.2))
    plt.bar(quarters, infra, label="Infra", color="#457B9D")
    plt.bar(quarters, data, bottom=infra, label="Data", color="#1D3557")
    stack_base = [infra[i] + data[i] for i in range(6)]
    plt.bar(quarters, support, bottom=stack_base, label="Support", color="#A8DADC")
    plt.plot(quarters, optimized_overlay, marker="o", linewidth=2.2, color="#E63946", label="Post-optimization")
    plt.title("Cost per Tenant by Quarter (kUSD)")
    plt.ylabel("kUSD")
    plt.grid(axis="y", alpha=0.25)
    plt.legend(loc="upper left")
    _finalize_figure(path, "png")


def _chart_delivery(path: Path, rng: random.Random) -> None:
    periods = [f"S{i}" for i in range(1, 11)]
    deploy = [
        7 + i + (2 if i >= 5 else 0) + rng.uniform(-0.6, 0.6)
        for i in range(10)
    ]
    lead_days = [
        max(2.2, 14 - 0.9 * i - (1.8 if i >= 5 else 0) + rng.uniform(-0.5, 0.5))
        for i in range(10)
    ]

    fig, ax1 = plt.subplots(figsize=(8.6, 4.2))
    ax1.plot(periods, deploy, color="#2A9D8F", marker="o", linewidth=2.2)
    ax1.set_ylabel("Deployments / sprint", color="#2A9D8F")
    ax1.tick_params(axis="y", labelcolor="#2A9D8F")
    ax1.grid(alpha=0.22)

    ax2 = ax1.twinx()
    ax2.plot(periods, lead_days, color="#E76F51", marker="s", linewidth=2.0)
    ax2.set_ylabel("Lead time (days)", color="#E76F51")
    ax2.tick_params(axis="y", labelcolor="#E76F51")

    plt.title("Delivery Throughput and Lead-Time Shift")
    _finalize_figure(path, "png")


def _chart_incidents(path: Path, rng: random.Random) -> None:
    release_cadence = [
        3.2,
        3.8,
        4.1,
        4.6,
        5.2,
        5.8,
        6.0,
        6.5,
        6.8,
        7.2,
        7.6,
        8.0,
    ]
    incidents = [
        17.5 - 1.55 * x + rng.uniform(-1.1, 1.1)
        for x in release_cadence
    ]

    n = len(release_cadence)
    sx = sum(release_cadence)
    sy = sum(incidents)
    sxy = sum(release_cadence[i] * incidents[i] for i in range(n))
    sxx = sum(x * x for x in release_cadence)
    slope = (n * sxy - sx * sy) / (n * sxx - sx * sx)
    intercept = (sy - slope * sx) / n

    trend_x = [min(release_cadence), max(release_cadence)]
    trend_y = [slope * x + intercept for x in trend_x]

    plt.figure(figsize=(8.6, 4.2))
    plt.scatter(release_cadence, incidents, color="#1D3557", s=48, alpha=0.85, label="Observed")
    plt.plot(trend_x, trend_y, color="#E63946", linewidth=2.0, label="Trendline")
    plt.title("Incidents vs Release Cadence")
    plt.xlabel("Releases per month")
    plt.ylabel("P1/P2 incidents per quarter")
    plt.grid(alpha=0.22)
    plt.legend(loc="upper right")
    _finalize_figure(path, "jpg")


def _chart_roadmap(path: Path, rng: random.Random) -> None:
    milestones = [
        "Platform baseline",
        "Service extraction",
        "Tenant costing",
        "Security hardening",
        "Legacy bridge",
        "Scaled rollout",
    ]
    complete = [18, 31, 44, 57, 72, 86]
    forecast = [
        complete[i] + (9 if i < 2 else 6 if i < 4 else 4) + rng.uniform(-1.5, 1.5)
        for i in range(6)
    ]

    y = list(range(len(milestones)))
    plt.figure(figsize=(8.6, 4.6))
    plt.barh(y, forecast, color="#A8DADC", label="Forecast")
    plt.barh(y, complete, color="#457B9D", label="Completed")
    plt.yticks(y, milestones)
    plt.xlim(0, 100)
    plt.xlabel("Progress (%)")
    plt.title("Migration Roadmap Burn-Up")
    plt.grid(axis="x", alpha=0.22)
    plt.legend(loc="lower right")
    _finalize_figure(path, "png")


def _to_hex(path: Path) -> str:
    return path.read_bytes().hex().upper()


def generate_chart_assets(seed: int, temp_dir: Path) -> list[ChartAsset]:
    rng = random.Random(seed)

    chart_paths = {
        "ram": temp_dir / "ram_profile.png",
        "cost": temp_dir / "cost_per_tenant.png",
        "delivery": temp_dir / "delivery_shift.png",
        "incidents": temp_dir / "incidents_vs_cadence.jpg",
        "roadmap": temp_dir / "roadmap_burnup.png",
    }

    _chart_ram(chart_paths["ram"], rng)
    _chart_cost(chart_paths["cost"], rng)
    _chart_delivery(chart_paths["delivery"], rng)
    _chart_incidents(chart_paths["incidents"], rng)
    _chart_roadmap(chart_paths["roadmap"], rng)

    return [
        ChartAsset("ram", "pngblip", _to_hex(chart_paths["ram"]), 6400, 3000, 100, 100),
        ChartAsset("cost", "pngblip", _to_hex(chart_paths["cost"]), 6400, 3000, 100, 100),
        ChartAsset("delivery", "pngblip", _to_hex(chart_paths["delivery"]), 6400, 3000, 100, 100),
        ChartAsset("incidents", "jpegblip", _to_hex(chart_paths["incidents"]), 6400, 3000, 100, 100),
        ChartAsset("roadmap", "pngblip", _to_hex(chart_paths["roadmap"]), 6400, 3200, 100, 100),
    ]


def emit_section(
    section_idx: int,
    title: str,
    bookmark: str,
    chart: ChartAsset,
    showcase_mode: bool,
) -> str:
    ix = section_idx + 1
    baseline = 86 + section_idx * 3
    current = baseline + 12
    defect_rate = max(6, 18 - section_idx)
    ownership_lag = 4 + (section_idx % 5)

    align_control = ["\\ql", "\\qc", "\\qr", "\\qj"][section_idx % 4]

    lines: list[str] = []
    lines.append("{\\*\\bkmkstart " + bookmark + "}{\\*\\bkmkend " + bookmark + "}\n")
    lines.append("\\pard\\sb220\\sa120\\fs30\\b " + escape_rtf_text(f"{ix}. {title}") + "\\b0\\par\n")
    lines.append(
        "\\pard"
        + align_control
        + "\\fs21 "
        + escape_rtf_text(
            "This section summarizes practical modernization tradeoffs across product delivery, "
            "platform ownership, and cloud economics."
        )
        + "\\par\n"
    )
    lines.append(
        "\\pard\\qj\\fs20 "
        + "Structured findings include "
        + "{\\b measurable delivery acceleration\\b0}, "
        + "{\\i tighter operational feedback loops\\i0}, "
        + "{\\ul explicit cost attribution\\ulnone}, "
        + "and a plan to replace {\\strike fragmented release orchestration\\strike0}. "
        + "The target operating model remains {\\caps standards-led\\caps0} and "
        + "{\\scaps cloud-ready\\scaps0}."
        + "\\par\n"
    )
    lines.append(
        "\\pard\\qj\\f1\\fs22\\cf2 "
        + escape_rtf_text(
            "Program note: the transition must preserve service continuity while reducing coupling "
            "between framework internals and product delivery teams."
        )
        + "\\cf1\\f0\\fs20\\par\n"
    )
    lines.append(
        "\\pard\\qj\\cbpat7\\cfpat2\\shading2500 "
        + escape_rtf_text(
            "Shaded insight: overprovisioned shared runtime pools hide unit economics and reduce "
            "incentives for efficiency by service ownership."
        )
        + "\\par\n"
    )
    lines.append(
        "\\pard\\plain\\qj\\fs20\\cb6 Background-only text after plain reset. "
        "\\highlight5 Highlight wins when both controls exist.\\highlight0 "
        "Back to background color.\\par\n"
    )
    lines.append(
        "\\pard\\qj\\fs20 Unicode marker check: \\u8212? modernization \\u8364? and \\u169? policy baseline.\\par\n"
    )

    lines.append(
        "{\\field{\\*\\fldinst HYPERLINK \"https://example.com/northstar/modernization/section-"
        + str(ix)
        + "\"}{\\fldrslt Open technical appendix "
        + str(ix)
        + "}}\\par\n"
    )
    lines.append(
        "{\\field{\\*\\fldinst HYPERLINK \"mailto:platform-office@example.com\"}{\\fldrslt Contact platform office}}\\par\n"
    )
    if section_idx > 0:
        lines.append(
            "{\\field{\\*\\fldinst HYPERLINK \\l \"sec_exec_summary\"}{\\fldrslt Return to Executive Summary}}\\par\n"
        )

    lines.append(embed_pict(chart.blip_kind, chart.hex_payload, chart.width_twips, chart.height_twips, chart.scalex, chart.scaley) + "\\par\n")

    lines.append("\\pard\\ls1\\ilvl0 Primary finding " + str(ix) + ": delivery throughput grew while operational risk remained bounded.\\par\n")
    lines.append("\\pard\\ls1\\ilvl1 Supporting evidence A." + str(ix) + ": demand spikes were absorbed via autoscaling policy.\\par\n")
    lines.append("\\pard\\ls1\\ilvl1 Supporting evidence B." + str(ix) + ": tenant cost variance became measurable per service.\\par\n")
    lines.append("\\pard\\ls2\\ilvl0 Action item " + str(ix) + ": publish target-state API contracts and migration checkpoints.\\par\n")

    lines.extend(
        [
            "\\trowd\\trgaph108\\trleft0\\trql\\trcbpat8\\trshdng2500\\trcfpat2"
            "\\trbrdrt\\brdrs\\brdrw8\\brdrcf2\\trbrdrl\\brdrs\\brdrw8\\brdrcf2"
            "\\trbrdrb\\brdrs\\brdrw8\\brdrcf2\\trbrdrr\\brdrs\\brdrw8\\brdrcf2"
            "\\clvertalc\\clcbpat7\\clcfpat2\\clshdng2500\\clbrdrt\\brdrdot\\brdrw4\\clbrdrl\\brdrs\\brdrw4\\clbrdrb\\brdrs\\brdrw4\\clbrdrr\\brdrs\\brdrw4\\cellx2500"
            "\\clmgf\\clvertalc\\clcbpat6\\clbrdrt\\brdrdb\\brdrw6\\clbrdrl\\brdrs\\brdrw4\\clbrdrb\\brdrdash\\brdrw4\\clbrdrr\\brdrs\\brdrw4\\cellx7600"
            "\\clmrg\\clvertalb\\clcbpat6\\clbrdrt\\brdrs\\brdrw4\\clbrdrl\\brdrs\\brdrw4\\clbrdrb\\brdrs\\brdrw4\\clbrdrr\\brdrs\\brdrw4\\cellx9800"
            "\\clvertalt\\clcbpat7\\clbrdrt\\brdrs\\brdrw4\\clbrdrl\\brdrs\\brdrw4\\clbrdrb\\brdrs\\brdrw4\\clbrdrr\\brdrnil\\cellx11600",
            "\\intbl Metric\\cell Baseline vs Current\\cell \\cell Notes\\cell\\row",
            "\\trowd\\trgaph108\\trleft0\\trqc\\clvertalt\\clvmgf\\cellx2500\\cellx7600\\cellx9800\\cellx11600",
            "\\intbl Throughput\\cell " + str(baseline) + " -> " + str(current) + "\\cell cadence stabilized\\cell Governance owner assigned\\cell\\row",
            "\\trowd\\trgaph108\\trleft0\\trqr\\clvertalb\\clvmrg\\cellx2500\\cellx7600\\cellx9800\\cellx11600",
            "\\intbl \\cell Defect rate " + str(defect_rate) + "\\cell targeted hardening\\cell cross-team review in " + str(ownership_lag) + " weeks\\cell\\row",
        ]
    )

    if section_idx % 3 == 0:
        lines.append(
            "\\pard\\fs20 Governance checkpoint "
            + str(ix)
            + " confirmed with legal and operations sign-off"
            + "{\\footnote \\pard\\plain\\fs18 Footnote "
            + str(ix)
            + ": Shared ownership remains acceptable when service boundaries and incident budgets are explicit.}"
            + ".\\par\n"
        )

    if section_idx % 4 == 1:
        lines.append(
            "\\pard\\fs20 Strategic note "
            + str(ix)
            + " includes staged compliance rollout"
            + "{\\endnote \\pard\\plain\\fs18 Endnote "
            + str(ix)
            + ": Modular certification reduces audit blast radius and shortens remediation lead-time.}"
            + ".\\par\n"
        )

    if section_idx == 6:
        lines.extend(
            [
                "\\pard\\sb120\\sa60\\fs22\\b Nested Table and Embedded Image Showcase\\b0\\par",
                "\\trowd\\cellx11600",
                "\\intbl Outer narrative before nested controls.\\par",
                "\\itap2\\nesttableprops\\trowd\\cellx5800\\cellx11600",
                "\\intbl Nested-A\\nestcell Nested-B\\nestcell\\nestrow",
                "\\itap1",
                "\\intbl " + embed_pict(chart.blip_kind, chart.hex_payload, 3600, 1700, 85, 85) + "\\cell",
                "\\row",
            ]
        )

    if section_idx == 8:
        lines.extend(
            [
                "\\sect\\sectd\\lndscpsxn\\pgwsxn15840\\pghsxn12240\\marglsxn900\\margrsxn900",
                "\\pard\\sb140\\sa80\\fs24\\b Landscape Risk Matrix\\b0\\par",
                "\\trowd\\trgaph108\\trleft0\\cellx2200\\cellx4400\\cellx6600\\cellx8800\\cellx11000\\cellx13600",
                "\\intbl Region\\cell Owner\\cell Severity\\cell ETA\\cell Status\\cell Notes\\cell\\row",
                "\\trowd\\trgaph108\\trleft0\\cellx2200\\cellx4400\\cellx6600\\cellx8800\\cellx11000\\cellx13600",
                "\\intbl North America\\cell Platform Eng\\cell High\\cell 2026-09-15\\cell In Progress\\cell Legacy gateway dependency\\cell\\row",
                "\\trowd\\trgaph108\\trleft0\\cellx2200\\cellx4400\\cellx6600\\cellx8800\\cellx11000\\cellx13600",
                "\\intbl EMEA\\cell Ops Reliability\\cell Medium\\cell 2026-10-02\\cell Planned\\cell Security hardening window\\cell\\row",
                "\\sect\\sectd\\lndscpsxn0\\pgwsxn12240\\pghsxn15840\\marglsxn1440\\margrsxn1440",
            ]
        )

    if section_idx == 10:
        lines.append(
            "\\pard\\fs20 Reference check: roadmap details begin on page {\\field{\\*\\fldinst PAGEREF sec_implementation_roadmap}{\\fldrslt 8}}.\\par\n"
        )

    if showcase_mode and section_idx == 11:
        lines.append("\\pard\\fs20 Closing recommendation: keep legacy stable while modularizing high-change domains first.\\par\n")

    return "\n".join(lines) + "\n"


def build_document(
    *,
    pages: int,
    include_unsupported_destination: bool,
    title: str,
    subtitle: str,
    chart_assets: list[ChartAsset],
) -> str:
    out: list[str] = []

    out.append("{\\rtf1\\ansi\\ansicpg1252\\deff0\n")
    out.append("{\\fonttbl{\\f0 Calibri;}{\\f1 Cambria;}{\\f2 Consolas;}{\\f3 Arial;}}\n")
    out.append(
        "{\\colortbl;"
        "\\red0\\green0\\blue0;"
        "\\red31\\green73\\blue125;"
        "\\red220\\green53\\blue69;"
        "\\red34\\green139\\blue34;"
        "\\red255\\green193\\blue7;"
        "\\red242\\green242\\blue242;"
        "\\red224\\green240\\blue255;"
        "\\red245\\green245\\blue220;"
        "\\red90\\green90\\blue90;}\n"
    )
    out.append("\\paperw12240\\paperh15840\\margl1440\\margr1440\\margt1200\\margb1200\n")
    out.append(
        "{\\header \\pard\\qr\\fs18 Northstar Enterprise Systems\\tab Platform Modernization Whitepaper\\par}\n"
    )
    out.append(
        "{\\footer \\pard\\qc\\fs18 Page {\\field{\\*\\fldinst PAGE}{\\fldrslt 1}} of "
        "{\\field{\\*\\fldinst NUMPAGES}{\\fldrslt 1}} "
        "(section total {\\field{\\*\\fldinst SECTIONPAGES}{\\fldrslt 1}})\\par}\n"
    )
    if include_unsupported_destination:
        out.append("{\\*\\unsupporteddestination This destination is intentionally dropped in strict mode.}\n")

    out.append(
        "{\\listtable"
        "{\\list\\listtemplateid1\\listid1"
        "{\\listlevel\\levelnfc23\\levelnfcn23\\leveljc0\\levelfollow0\\levelstartat1\\levelspace0\\levelindent0"
        "{\\leveltext\\'01\\u8226?;}{\\levelnumbers;}\\fi-360\\li720}"
        "{\\listlevel\\levelnfc23\\levelnfcn23\\leveljc0\\levelfollow0\\levelstartat1\\levelspace0\\levelindent0"
        "{\\leveltext\\'01\\u9702?;}{\\levelnumbers;}\\fi-360\\li1440}"
        "{\\listname ;}\\listid1}"
        "{\\list\\listtemplateid2\\listid2"
        "{\\listlevel\\levelnfc0\\levelnfcn0\\leveljc0\\levelfollow0\\levelstartat1\\levelspace0\\levelindent0"
        "{\\leveltext\\'02\\'00.;}{\\levelnumbers\\'01;}\\fi-360\\li720}"
        "{\\listname ;}\\listid2}}\n"
    )
    out.append(
        "{\\listoverridetable{\\listoverride\\listid1\\listoverridecount0\\ls1}"
        "{\\listoverride\\listid2\\listoverridecount0\\ls2}}\n"
    )
    out.append("\\viewkind4\\uc1\n")

    out.append("\\pard\\qc\\sa220\\fs38\\b " + escape_rtf_text(title) + "\\b0\\par\n")
    out.append("\\pard\\qc\\fs22 " + escape_rtf_text(subtitle) + "\\par\n")
    out.append(
        "\\pard\\sb100\\sa120\\qj\\fs20 "
        + escape_rtf_text(
            "This corpus models a realistic modernization proposal for a legacy enterprise platform "
            "using a dual-track migration strategy with strict governance and measurable outcomes."
        )
        + "\\par\n"
    )
    out.append("{\\field{\\*\\fldinst TOC \\h}{\\fldrslt Table of contents placeholder}}\\par\n")
    out.append(
        "\\pard\\fs20 Quick links: "
        "{\\field{\\*\\fldinst HYPERLINK \\l \"sec_dual_track_strategy\"}{\\fldrslt Dual-Track Strategy}}"
        " | "
        "{\\field{\\*\\fldinst HYPERLINK \\l \"sec_implementation_roadmap\"}{\\fldrslt Implementation Roadmap}}"
        " | "
        "{\\field{\\*\\fldinst HYPERLINK \\l \"sec_risk_register\"}{\\fldrslt Risk Register}}"
        "\\par\n"
    )

    out.append(
        "\\pard\\sb120\\sa80\\fs20 "
        "Formatting showcase: {\\b bold\\b0}, {\\i italic\\i0}, {\\ul underline\\ulnone}, "
        "{\\strike strike\\strike0}, {\\caps all caps\\caps0}, {\\scaps small caps\\scaps0}, "
        "and {\\f2\\cf2\\fs18 monospace accent run\\f0\\cf1\\fs20}."
        "\\par\n"
    )

    for page_idx in range(pages):
        section_title, bookmark = SECTIONS[page_idx % len(SECTIONS)]
        chart = chart_assets[page_idx % len(chart_assets)]
        out.append(
            emit_section(
                page_idx,
                section_title,
                bookmark,
                chart,
                showcase_mode=not include_unsupported_destination,
            )
        )
        if page_idx < pages - 1:
            out.append("\\page\n")

    out.append("}\n")
    return "".join(out)


def write_meta(
    path: Path,
    fixture_name: str,
    description: str,
    expected_non_strict_exit: int,
    expected_strict_exit: int,
    required_warning_types: Iterable[str],
    required_dropped_reasons: Iterable[str],
    min_docx_bytes: int,
    min_html_bytes: int,
) -> None:
    payload = {
        "fixture": fixture_name,
        "description": description,
        "expected_non_strict_exit": expected_non_strict_exit,
        "expected_strict_exit": expected_strict_exit,
        "required_warning_types": list(required_warning_types),
        "required_dropped_reasons": list(required_dropped_reasons),
        "min_docx_bytes": min_docx_bytes,
        "min_html_bytes": min_html_bytes,
    }
    path.write_text(json.dumps(payload, indent=2) + "\n", encoding="utf-8")


def parse_args() -> argparse.Namespace:
    script_root = Path(__file__).resolve().parents[1]
    default_out_dir = script_root / "fixtures" / "realworld"

    parser = argparse.ArgumentParser(description="Generate platform modernization realworld fixtures")
    parser.add_argument("--seed", type=int, default=DEFAULT_SEED, help="Deterministic RNG seed")
    parser.add_argument(
        "--out-dir",
        type=Path,
        default=default_out_dir,
        help="Output directory for .rtf and .meta.json files",
    )
    parser.add_argument(
        "--pages-showcase",
        type=int,
        default=DEFAULT_SHOWCASE_PAGES,
        help="Page count for strict-pass showcase fixture",
    )
    parser.add_argument(
        "--pages-warning-probe",
        type=int,
        default=DEFAULT_WARNING_PAGES,
        help="Page count for strict-fail warning-probe fixture",
    )
    return parser.parse_args()


def main() -> None:
    args = parse_args()
    out_dir: Path = args.out_dir
    out_dir.mkdir(parents=True, exist_ok=True)

    with tempfile.TemporaryDirectory(prefix="platform_modernization_assets_") as td:
        chart_assets = generate_chart_assets(args.seed, Path(td))

        showcase_name = "platform_modernization_showcase_12p.rtf"
        warning_name = "platform_modernization_warning_probe_10p.rtf"

        showcase_rtf = build_document(
            pages=args.pages_showcase,
            include_unsupported_destination=False,
            title="Platform Modernization Proposal",
            subtitle=(
                "Dual-track transition from a legacy unified framework to a modular cloud-native platform"
            ),
            chart_assets=chart_assets,
        )
        warning_rtf = build_document(
            pages=args.pages_warning_probe,
            include_unsupported_destination=True,
            title="Platform Modernization Proposal (Warning Probe)",
            subtitle=(
                "Companion corpus to validate strict-mode fail-closed behavior with controlled dropped content"
            ),
            chart_assets=chart_assets,
        )

        (out_dir / showcase_name).write_text(showcase_rtf, encoding="utf-8")
        (out_dir / warning_name).write_text(warning_rtf, encoding="utf-8")

        write_meta(
            out_dir / "platform_modernization_showcase_12p.meta.json",
            fixture_name=showcase_name,
            description=(
                "Anonymized platform modernization proposal with deterministic charts, "
                "broad RTF feature coverage, and strict-mode pass behavior."
            ),
            expected_non_strict_exit=0,
            expected_strict_exit=0,
            required_warning_types=[],
            required_dropped_reasons=[],
            min_docx_bytes=437020,
            min_html_bytes=679384,
        )
        write_meta(
            out_dir / "platform_modernization_warning_probe_10p.meta.json",
            fixture_name=warning_name,
            description=(
                "Companion modernization proposal that intentionally includes an unsupported destination "
                "to validate strict-mode fail-closed behavior."
            ),
            expected_non_strict_exit=0,
            expected_strict_exit=4,
            required_warning_types=["dropped_content"],
            required_dropped_reasons=["Dropped unknown destination group \\unsupporteddestination"],
            min_docx_bytes=409668,
            min_html_bytes=579139,
        )

    print("Generated fixtures:")
    for file_name in [
        showcase_name,
        "platform_modernization_showcase_12p.meta.json",
        warning_name,
        "platform_modernization_warning_probe_10p.meta.json",
    ]:
        print(f" - {out_dir / file_name}")


if __name__ == "__main__":
    main()
