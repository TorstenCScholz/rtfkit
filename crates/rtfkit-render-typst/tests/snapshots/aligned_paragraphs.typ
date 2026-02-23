// rtfkit style profile: report

#set text(
  font: ("Libertinus Serif"),
  size: 11pt,
  fill: rgb("1A1A1A"),
  lang: "en",
)

#set par(
  leading: 1.6em,
  spacing: 12pt,
)

// Heading styles
#show heading.where(level: 1): it => {
  set text(font: ("Libertinus Serif"), size: 26pt, weight: 700)
  set block(above: 28pt, below: 14pt)
  it
}

#show heading.where(level: 2): it => {
  set text(font: ("Libertinus Serif"), size: 20pt, weight: 600)
  set block(above: 22pt, below: 12pt)
  it
}

#show heading.where(level: 3): it => {
  set text(font: ("Libertinus Serif"), size: 15pt, weight: 600)
  set block(above: 16pt, below: 10pt)
  it
}

// Link styles
#show link: set text(fill: rgb("2563EB"))

// Table styles
#set table(
  stroke: 0.5pt + rgb("D1D5DB"),
  inset: (x: 8pt, y: 5pt),
  fill: (x, y) => {
    if y == 0 {
      rgb("E6E9ED")
    } else if calc.rem(y, 2) == 1 {
      rgb("F4F6F8")
    } else {
      none
    }
  },
)

// Table header emphasis
#show table.cell.where(y: 0): set text(weight: 600)

// List styles
#set list(
  indent: 20pt,
  body-indent: 8pt,
  spacing: 6pt,
)

#set enum(
  indent: 20pt,
  body-indent: 8pt,
  spacing: 6pt,
)

#set page(
  width: 210mm,
  height: 297mm,
  margin: (top: 25mm, bottom: 25mm, left: 22mm, right: 22mm),
)

Left-aligned text

#align(center)[Centered text]

#align(right)[Right-aligned text]

#align(justify)[Justified text]