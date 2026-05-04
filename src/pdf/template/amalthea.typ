// Invoice template (Amalthea: Editorial & refined) -- expects `data` variable to be defined

// ─── Palette ───
#let accent     = rgb(data.branding.accent_color)
#let ink        = rgb("#16171a")
#let ink-soft   = rgb("#4a4d56")
#let ink-muted  = rgb("#8a8e98")
#let rule-color = rgb("#e8e5dd")

#set page(
  paper: "a4",
  margin: (top: 1.6cm, bottom: 1.6cm, left: 2.2cm, right: 2.2cm),
  footer: if "footer_text" in data.branding and data.branding.footer_text != "" {
    align(center)[
      #line(length: 100%, stroke: 0.3pt + rule-color)
      #v(0.25cm)
      #text(size: 8pt, fill: ink-muted)[#data.branding.footer_text]
    ]
  },
)

#set text(font: data.branding.font, size: 10pt)

// ─── Font stacks (broad fallbacks; Typst picks the first available) ───
#let serif-font = ("Iowan Old Style", "Source Serif 4", "Source Serif Pro", "Georgia", "Times New Roman", "DejaVu Serif")
#let mono-font  = ("JetBrains Mono", "SF Mono", "Menlo", "Courier New", "DejaVu Sans Mono", "Liberation Mono")

// ─── Reusable small-caps label ───
#let caps-label(s) = text(
  size: 7pt,
  weight: "semibold",
  tracking: 0.14em,
  fill: ink-muted,
)[#upper(s)]

// ═══ HEADER: serif title left, invoice number right ═══
#grid(
  columns: (1fr, auto),
  align: (left + bottom, right + bottom),
  text(font: serif-font, size: 36pt, weight: "semibold", tracking: -0.5pt, fill: ink)[Invoice],
  align(right)[
    #caps-label("Invoice no.") \
    #v(2pt)
    #text(size: 11pt, weight: "medium", fill: ink-soft)[#data.invoice.number]
  ],
)

#v(0.4cm)
#line(length: 100%, stroke: 0.3pt + rule-color)
#v(0.4cm)

// ═══ META STRIP: 4 columns ═══
#let meta-cell(label-text, value-content) = [
  #caps-label(label-text) \
  #v(2pt)
  #text(size: 10pt, fill: ink-soft)[#value-content]
]

#grid(
  columns: (1fr, 1fr, 1fr, 1fr),
  gutter: 1cm,
  meta-cell("Issued",  data.invoice.date),
  meta-cell("Due",     data.invoice.due_date),
  meta-cell("Period",  data.invoice.period),
  meta-cell("Amount", [#data.invoice.currency #data.invoice.total]),
)

#v(0.4cm)
#line(length: 100%, stroke: 0.3pt + rule-color)
#v(0.5cm)

// ═══ PARTIES: From / Billed To ═══
#grid(
  columns: (1fr, 1fr),
  gutter: 1.4cm,
  // From
  [
    #caps-label("From")
    #v(4pt)
    #text(font: serif-font, size: 13pt, weight: "medium", fill: ink)[#data.sender.name]
    #v(3pt)
    #set text(size: 10pt, fill: ink-soft)
    #for ln in data.sender.address [
      #ln \
    ]
    #data.sender.email
  ],
  // Billed To
  [
    #caps-label("Billed to")
    #v(4pt)
    #text(font: serif-font, size: 13pt, weight: "medium", fill: ink)[#data.recipient.name]
    #v(3pt)
    #set text(size: 10pt, fill: ink-soft)
    #for ln in data.recipient.address [
      #ln \
    ]
    #if "company_id" in data.recipient or "vat_number" in data.recipient {
      v(2pt)
      block[
        #set text(size: 8.5pt, fill: ink-muted)
        #if "company_id" in data.recipient [Company ID #data.recipient.company_id]
        #if "company_id" in data.recipient and "vat_number" in data.recipient [ · ]
        #if "vat_number" in data.recipient [VAT #data.recipient.vat_number]
      ]
    }
  ],
)

#v(0.6cm)

// ═══ LINE ITEMS ═══
#let head-cell(s) = table.cell(
  stroke: (bottom: 0.4pt + ink),
)[#text(size: 7pt, weight: "semibold", tracking: 0.14em, fill: ink-muted)[#upper(s)]]

#let body-strong(c) = table.cell(
  stroke: (bottom: 0.3pt + rule-color),
)[#text(size: 10pt, weight: "medium", fill: ink)[#c]]

#let body-muted(c) = table.cell(
  stroke: (bottom: 0.3pt + rule-color),
)[#text(size: 10pt, fill: ink-muted)[#c]]

#if data.invoice.has_tax {
  table(
    columns: (1fr, auto, auto, auto, auto, auto),
    align: (left, right, right, right, right, right),
    stroke: none,
    inset: (x: 8pt, y: 9pt),
    table.header(
      head-cell("Description"),
      head-cell("Days"),
      head-cell("Rate"),
      head-cell("Amount"),
      head-cell("Tax %"),
      head-cell("Tax"),
    ),
    ..for item in data.invoice.line_items {
      (
        body-strong(item.description),
        body-muted(item.days),
        body-muted(item.rate),
        body-strong(item.amount),
        body-muted(item.tax_rate),
        body-muted(item.tax_amount),
      )
    },
  )
} else {
  table(
    columns: (1fr, auto, auto, auto),
    align: (left, right, right, right),
    stroke: none,
    inset: (x: 8pt, y: 9pt),
    table.header(
      head-cell("Description"),
      head-cell("Days"),
      head-cell("Rate"),
      head-cell("Amount"),
    ),
    ..for item in data.invoice.line_items {
      (
        body-strong(item.description),
        body-muted(item.days),
        body-muted(item.rate),
        body-strong(item.amount),
      )
    },
  )
}

#v(0.3cm)

// ═══ SUBTOTAL / TAX (only when tax applies) ═══
#if data.invoice.has_tax {
  align(right)[
    #table(
      columns: (auto, auto),
      align: (right, right),
      stroke: none,
      inset: (x: 12pt, y: 4pt),
      text(size: 10pt, fill: ink-soft)[Subtotal],
      text(size: 10pt, fill: ink)[#data.invoice.currency #data.invoice.subtotal],
      text(size: 10pt, fill: ink-soft)[Tax],
      text(size: 10pt, fill: ink)[#data.invoice.currency #data.invoice.tax_total],
    )
  ]
}

#v(0.3cm)

// ═══ TOTAL DUE BAR (the visual anchor) ═══
#rect(fill: accent, width: 100%, inset: (x: 1cm, y: 0.5cm))[
  #grid(
    columns: (1fr, auto),
    align: (left + horizon, right + horizon),
    text(size: 8pt, weight: "semibold", tracking: 0.14em, fill: white.transparentize(35%))[#upper("Total due")],
    text(font: serif-font, size: 22pt, weight: "semibold", tracking: -0.3pt, fill: white)[#data.invoice.currency #data.invoice.total],
  )
]

#v(0.7cm)

// ═══ PAYMENT ═══
#caps-label("Payment")
#v(0.35cm)
#for (i, method) in data.payment.enumerate() [
  #if i > 0 [#v(0.4cm)]
  #if "label" in method [
    #text(size: 10pt, weight: "medium", fill: ink)[#method.label]
    #v(0.25cm)
  ]
  #table(
    columns: (auto, auto),
    align: (left, left),
    stroke: none,
    inset: (x: 0pt, y: 4pt),
    column-gutter: 1.5cm,
    text(size: 8.5pt, fill: ink-muted, tracking: 0.06em)[IBAN],
    text(font: mono-font, size: 9.5pt, fill: ink)[#method.iban],
    text(size: 8.5pt, fill: ink-muted, tracking: 0.06em)[BIC],
    text(font: mono-font, size: 9.5pt, fill: ink)[#method.bic_swift],
  )
]

