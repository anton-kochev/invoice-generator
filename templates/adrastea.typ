// Invoice template (Adrastea: Minimal & monospaced) -- expects `data` variable to be defined
//
// Grayscale by design: monospaced figures, hairline rules, two faint gray
// bands (table header, total row) and nothing else. `branding.accent_color`
// appears exactly once — the rule above the total — so the branding override
// is still honoured without breaking the restraint.
//
// Deliberately omits sender/recipient addresses and payment details; the
// document carries names, line items and the total only.
//
// @invoice-capabilities: units=days,hours mixed-units=yes quantity-column=yes
//
// The quantity column header lives outside the per-row loop, so it can only
// name one unit. When an invoice mixes days and hours the binary sends a
// neutral header ("Qty") and each quantity carries its own short suffix.

// Quantity and its unit. Lower-cased to match this template's all-lowercase
// labels. Older binaries send neither key; the defaults keep days.
#let unit-label = lower(data.invoice.at("unit_label", default: "Days"))
#let item-qty(item) = item.at("quantity", default: item.days)
#let item-unit(item) = item.at("unit", default: "days")

#let mixed-units = data.invoice.line_items.map(item-unit).dedup().len() > 1

// Hours read better as 2:45 than as 2.75, so the decimal is converted here.
// Purely presentational: the binary bills in decimal hours and every money
// figure on the page still derives from that, so a quantity whose decimal
// doesn't land on a whole minute (2.33 -> 2:20) shows the rounded clock time
// beside an amount computed from the unrounded value.
//
// The quantity arrives pre-formatted for the invoice locale -- "2.75",
// "2,75", or "1 234,50" with a non-breaking thousands separator. The final
// "." or "," is therefore always the decimal point and everything before it
// is grouping noise, which is what makes this locale-agnostic.
#let hours-to-hmm(text) = {
  let negative = text.trim().starts-with("-")
  let parts = text.split(regex("[.,]"))
  let frac-str = if parts.len() > 1 { parts.last() } else { "" }
  let int-parts = if parts.len() > 1 { parts.slice(0, -1) } else { parts }
  let int-str = int-parts.join("").replace(regex("[^0-9]"), "")

  let hours = if int-str == "" { 0 } else { int(int-str) }
  let minutes = if frac-str == "" {
    0
  } else {
    int(calc.round(int(frac-str) / calc.pow(10, frac-str.len()) * 60))
  }
  // Two decimals can only reach 59.4 minutes, but carry anyway so a future
  // binary sending more precision can't print "2:60".
  if minutes >= 60 {
    hours += 1
    minutes -= 60
  }

  let mm = if minutes < 10 { "0" + str(minutes) } else { str(minutes) }
  (if negative { "-" } else { "" }) + str(hours) + ":" + mm
}

// Only rendered when the header can't speak for every row. Unknown units fall
// back to their own name, so a future unit still prints something truthful.
#let unit-suffix = (days: "d", hours: "h")
#let qty-cell(item) = {
  let unit = item-unit(item)
  let qty = if unit == "hours" { hours-to-hmm(item-qty(item)) } else { item-qty(item) }
  if mixed-units { qty + " " + unit-suffix.at(unit, default: unit) } else { qty }
}

#let accent = rgb(data.branding.accent_color)
#let ink = rgb("#111111")
#let ink-muted = rgb("#737373")
#let ink-faint = rgb("#a3a3a3")
#let rule-color = rgb("#dcdcdc")
#let band = rgb("#f6f7f8")

// Monospace stack for every figure on the page, so columns align optically.
#let mono-font = ("SF Mono", "Menlo", "DejaVu Sans Mono", "Courier New", "monospace")

#set page(
  paper: "a4",
  margin: (top: 2.4cm, bottom: 2.4cm, left: 2.2cm, right: 2.2cm),
)

#set text(font: data.branding.font, size: 9.5pt, fill: ink)

// Lowercase micro-label — deliberately not `upper()`.
#let label(body) = text(size: 7pt, tracking: 0.14em, fill: ink-faint)[#body]

// Every numeric string goes through this so figures share one face.
#let num(body, size: 9pt, fill: ink) = text(font: mono-font, size: size, fill: fill)[#body]

// --- Masthead: title line left, issue/due rows right ---
#grid(
  columns: (1fr, auto),
  align: (left + top, right + top),
  column-gutter: 1.5cm,
  [
    #if "logo_file" in data.branding [
      #image(data.branding.logo_file, height: 1cm)
      #v(0.4cm)
    ]
    #text(size: 18pt, fill: ink-faint)[invoice]#h(0.3cm)#num(
      data.invoice.number,
      size: 18pt,
    )
  ],
  [
    // label : value rows, values sharing a right edge
    #grid(
      columns: (auto, auto),
      column-gutter: 0.7cm,
      row-gutter: 0.15cm,
      align: (left, right),
      text(size: 8.5pt, fill: ink-muted)[issue date],
      num(data.invoice.date, size: 8.5pt),
      text(size: 8.5pt, fill: ink-muted)[due date],
      num(data.invoice.due_date, size: 8.5pt),
    )
  ],
)

#v(1.2cm)

// --- Parties: names only, both stacked on the left. No rules, no boxes ---
#label("from")
#v(0.1cm)
#data.sender.name

#v(0.4cm)

#label("to")
#v(0.1cm)
#data.recipient.name

#v(1.4cm)

// --- Line items: banded header, hairline under each row ---
#let head(body) = table.cell(fill: band, stroke: none)[#label(body)]
#let text-cell(body) = table.cell(stroke: (bottom: 0.5pt + rule-color))[#body]
#let figure-cell(body) = table.cell(stroke: (bottom: 0.5pt + rule-color))[
  #num(body, size: 8.5pt)
]

#if data.invoice.has_tax {
  table(
    columns: (1fr, auto, auto, auto, auto, auto),
    align: (left, right, right, right, right, right),
    stroke: none,
    inset: (x: 8pt, y: 10pt),
    table.header(
      head("description"),
      head(unit-label),
      head("rate"),
      head("amount"),
      head("tax %"),
      head("tax"),
    ),
    ..for item in data.invoice.line_items {
      (
        text-cell(item.description),
        figure-cell(qty-cell(item)),
        figure-cell(item.rate),
        figure-cell(item.amount),
        figure-cell(item.tax_rate),
        figure-cell(item.tax_amount),
      )
    },
  )
} else {
  table(
    columns: (1fr, auto, auto, auto),
    align: (left, right, right, right),
    stroke: none,
    inset: (x: 8pt, y: 10pt),
    table.header(
      head("description"),
      head(unit-label),
      head("rate"),
      head("amount"),
    ),
    ..for item in data.invoice.line_items {
      (
        text-cell(item.description),
        figure-cell(qty-cell(item)),
        figure-cell(item.rate),
        figure-cell(item.amount),
      )
    },
  )
}

#v(0.3cm)

// --- Summary: label/value rows, accent hairline + band on the total ---
#align(right)[
  #block(width: 8cm)[
    #if data.invoice.has_tax [
      #grid(
        columns: (1fr, auto),
        column-gutter: 1cm,
        row-gutter: 0.25cm,
        inset: (x: 8pt),
        text(size: 8.5pt, fill: ink-muted)[subtotal],
        align(right)[#num(data.invoice.subtotal, size: 8.5pt)],
        text(size: 8.5pt, fill: ink-muted)[tax],
        align(right)[#num(data.invoice.tax_total, size: 8.5pt)],
      )
      #v(0.3cm)
    ]
    #block(
      fill: band,
      stroke: (top: 1pt + accent),
      inset: (x: 8pt, y: 9pt),
      width: 100%,
    )[
      #grid(
        columns: (1fr, auto),
        align: (left + bottom, right + bottom),
        label("total"),
        [
          #num(data.invoice.total, size: 13pt)#h(0.15cm)#text(
            size: 8.5pt,
            fill: ink-faint,
          )[#data.invoice.currency]
        ],
      )
    ]
  ]
]

// --- Footer: only if footer_text is set; pushed to page bottom ---
#if "footer_text" in data.branding [
  #v(1fr)
  #align(center)[
    #label(data.branding.footer_text)
  ]
]
