// Invoice template (Europa: Designed minimal) -- expects `data` variable to be defined
//
// Three-point accent strategy:
//   1. Full-bleed 3pt brand bar across the top of every page
//   2. Line-items table header tinted at 10% accent opacity
//   3. 2pt accent rule above the Total in the summary block
// Everything else is grayscale typography. Restraint is the design.

#let accent = rgb(data.branding.accent_color)
#let ink = rgb("#1a1a1a")
#let ink-muted = rgb("#6b7280")
#let ink-faint = rgb("#9ca3af")
#let rule-color = rgb("#e5e7eb")
// Opaque 10%-tint of accent (computed, not alpha-blended) — avoids the
// vertical seams that appear at cell boundaries when an alpha-fill is
// rendered per cell.
#let header-fill = color.mix((white, 90%), (accent, 10%))

#set page(
  paper: "a4",
  margin: (top: 2.5cm, bottom: 2.5cm, left: 2.2cm, right: 2.2cm),
  background: align(top, block(fill: accent, height: 3pt, width: 100%)),
)

#set text(font: data.branding.font, size: 10pt, fill: ink)

#let label(body) = text(
  size: 7.5pt,
  weight: "medium",
  tracking: 0.08em,
  fill: ink-muted,
)[#upper(body)]

// --- Masthead: invoice identity (left) and dates (right) ---
#grid(
  columns: (1fr, auto),
  align: (left + bottom, right + bottom),
  column-gutter: 1.5cm,
  [
    #if "logo_file" in data.branding [
      #image(data.branding.logo_file, height: 1cm)
      #v(0.4cm)
    ]
    #label("Invoice")
    #v(0.2cm)
    #text(size: 22pt, weight: 600, tracking: -0.01em)[#data.invoice.number]
  ],
  [
    #grid(
      columns: (auto, auto),
      column-gutter: 1cm,
      align: (right, right),
      [
        #label("Issued")
        #v(0.15cm)
        #text(size: 10pt, weight: "medium")[#data.invoice.date]
      ],
      [
        #label("Due")
        #v(0.15cm)
        #text(size: 10pt, weight: "medium")[#data.invoice.due_date]
      ],
    )
  ],
)

#v(1.6cm)

// --- Parties: From / Billed to ---
#grid(
  columns: (1fr, 1fr),
  column-gutter: 1.5cm,
  [
    #label("Billed from")
    #v(0.2cm)
    #text(weight: "medium")[#data.sender.name]
    #v(0.1cm)
    #set text(size: 9pt, fill: ink-muted)
    #for ln in data.sender.address [
      #ln \
    ]
    #if data.sender.email != "" [
      #data.sender.email
    ]
  ],
  [
    #label("Billed to")
    #v(0.2cm)
    #text(weight: "medium")[#data.recipient.name]
    #v(0.1cm)
    #set text(size: 9pt, fill: ink-muted)
    #for ln in data.recipient.address [
      #ln \
    ]
    #if "company_id" in data.recipient [
      Co. ID #data.recipient.company_id \
    ]
    #if "vat_number" in data.recipient [
      VAT #data.recipient.vat_number
    ]
  ],
)

#v(1.8cm)

// --- Line items: tinted header, hairline row dividers, no borders inside header ---
#let head(body) = table.cell(fill: header-fill, stroke: none)[#label(body)]

#if data.invoice.has_tax {
  table(
    columns: (1fr, auto, auto, auto, auto, auto),
    align: (left, right, right, right, right, right),
    stroke: none,
    inset: (x: 8pt, y: 12pt),
    table.header(
      head("Description"),
      head("Days"),
      head("Rate"),
      head("Amount"),
      head("Tax %"),
      head("Tax"),
    ),
    ..for item in data.invoice.line_items {
      (
        table.cell(stroke: (bottom: 0.5pt + rule-color))[#item.description],
        table.cell(stroke: (bottom: 0.5pt + rule-color))[#item.days],
        table.cell(stroke: (bottom: 0.5pt + rule-color))[#item.rate],
        table.cell(stroke: (bottom: 0.5pt + rule-color))[#item.amount],
        table.cell(stroke: (bottom: 0.5pt + rule-color))[#item.tax_rate],
        table.cell(stroke: (bottom: 0.5pt + rule-color))[#item.tax_amount],
      )
    },
  )
} else {
  table(
    columns: (1fr, auto, auto, auto),
    align: (left, right, right, right),
    stroke: none,
    inset: (x: 8pt, y: 12pt),
    table.header(
      head("Description"),
      head("Days"),
      head("Rate"),
      head("Amount"),
    ),
    ..for item in data.invoice.line_items {
      (
        table.cell(stroke: (bottom: 0.5pt + rule-color))[#item.description],
        table.cell(stroke: (bottom: 0.5pt + rule-color))[#item.days],
        table.cell(stroke: (bottom: 0.5pt + rule-color))[#item.rate],
        table.cell(stroke: (bottom: 0.5pt + rule-color))[#item.amount],
      )
    },
  )
}

#v(0.8cm)

// --- Summary: right-aligned, 7cm wide, accent rule above Total ---
#align(right)[
  #block(width: 7cm)[
    #if data.invoice.has_tax [
      #grid(
        columns: (1fr, auto),
        column-gutter: 1cm,
        row-gutter: 0.25cm,
        text(fill: ink-muted)[Subtotal], align(right)[#data.invoice.subtotal],
        text(fill: ink-muted)[Tax], align(right)[#data.invoice.tax_total],
      )
      #v(0.3cm)
    ]
    #line(length: 100%, stroke: 2pt + accent)
    #v(0.3cm)
    #align(right)[
      #label("Total")
      #v(0.15cm)
      #text(size: 18pt, weight: 600)[#data.invoice.total]#h(0.15cm)#text(
        size: 9pt,
        fill: ink-faint,
        tracking: 0.02em,
      )[#data.invoice.currency]
    ]
  ]
]

#v(1.8cm)

// --- Payment: only if non-empty ---
#if data.payment.len() > 0 [
  #label("Pay to")
  #v(0.2cm)
  #for method in data.payment {
    block(below: 4pt)[
      #set text(size: 9pt, fill: ink-muted)
      #if "label" in method [
        #text(weight: "medium", fill: ink)[#method.label]#h(0.25cm)#text(fill: ink-faint)[·]#h(0.25cm)
      ]
      IBAN~#method.iban#h(0.25cm)#text(fill: ink-faint)[·]#h(0.25cm)BIC~#method.bic_swift
    ]
  }
]

// --- Footer: only if footer_text is set; pushed to page bottom ---
#if "footer_text" in data.branding [
  #v(1fr)
  #align(center)[
    #label(data.branding.footer_text)
  ]
]
