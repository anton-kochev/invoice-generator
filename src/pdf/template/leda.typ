// Invoice template (Leda: Modern & minimalist) -- expects `data` variable to be defined

#set page(
  paper: "a4",
  margin: (top: 2.5cm, bottom: 2.5cm, left: 2.8cm, right: 2.8cm),
)

#set text(font: data.branding.font, size: 10pt)

#let accent = rgb(data.branding.accent_color)
#let muted = rgb("#888888")

// --- Display header: huge INVOICE left, logo or invoice number right ---
#grid(
  columns: (1fr, auto),
  align: (left + horizon, right + horizon),
  [
    #text(42pt, weight: "bold", tracking: -1pt)[Invoice]
  ],
  [
    #if "logo_file" in data.branding {
      image(data.branding.logo_file, height: 1.6cm)
    } else {
      text(22pt, weight: "light", fill: muted)[\##data.invoice.number]
    }
  ],
)

#v(0.3cm)

// --- Meta strip ---
#grid(
  columns: (1fr, 1fr, 1fr, 1fr),
  gutter: 0.5cm,
  [
    #text(7pt, fill: muted, tracking: 1pt)[NUMBER] \
    #v(0.1cm)
    #text(10pt)[#data.invoice.number]
  ],
  [
    #text(7pt, fill: muted, tracking: 1pt)[ISSUED] \
    #v(0.1cm)
    #text(10pt)[#data.invoice.date]
  ],
  [
    #text(7pt, fill: muted, tracking: 1pt)[DUE] \
    #v(0.1cm)
    #text(10pt)[#data.invoice.due_date]
  ],
  [
    #text(7pt, fill: muted, tracking: 1pt)[PERIOD] \
    #v(0.1cm)
    #text(10pt)[#data.invoice.period]
  ],
)

#v(1cm)
#line(length: 100%, stroke: 0.3pt + rgb("#dddddd"))
#v(1cm)

// --- FROM / TO stacked vertically, airy ---
#text(7pt, fill: muted, tracking: 1pt)[FROM]
#v(0.2cm)
#text(12pt, weight: "medium")[#data.sender.name]
#v(0.15cm)
#set text(size: 10pt, fill: rgb("#555555"))
#for ln in data.sender.address [
  #ln \
]
#data.sender.email

#v(1cm)

#text(7pt, fill: muted, tracking: 1pt)[BILLED TO]
#v(0.2cm)
#set text(size: 10pt, fill: black)
#text(12pt, weight: "medium")[#data.recipient.name]
#v(0.15cm)
#set text(size: 10pt, fill: rgb("#555555"))
#for ln in data.recipient.address [
  #ln \
]
#if "company_id" in data.recipient [
  Company ID: #data.recipient.company_id \
]
#if "vat_number" in data.recipient [
  VAT: #data.recipient.vat_number
]

#v(1.2cm)
#line(length: 100%, stroke: 0.3pt + rgb("#dddddd"))
#v(0.5cm)

// --- Line Items Table: no borders, generous padding, hairline rules only ---
#set text(fill: black)
#if data.invoice.has_tax {
  table(
    columns: (1fr, auto, auto, auto, auto, auto),
    align: (left, right, right, right, right, right),
    stroke: none,
    inset: (x: 4pt, y: 12pt),
    table.header(
      text(weight: "medium", size: 8pt, fill: muted, tracking: 1pt)[DESCRIPTION],
      text(weight: "medium", size: 8pt, fill: muted, tracking: 1pt)[DAYS],
      text(weight: "medium", size: 8pt, fill: muted, tracking: 1pt)[RATE],
      text(weight: "medium", size: 8pt, fill: muted, tracking: 1pt)[AMOUNT],
      text(weight: "medium", size: 8pt, fill: muted, tracking: 1pt)[TAX %],
      text(weight: "medium", size: 8pt, fill: muted, tracking: 1pt)[TAX],
    ),
    table.hline(stroke: 0.3pt + rgb("#dddddd")),
    ..for item in data.invoice.line_items {
      (
        table.cell(stroke: (bottom: 0.3pt + rgb("#eeeeee")))[#text(size: 10pt)[#item.description]],
        table.cell(stroke: (bottom: 0.3pt + rgb("#eeeeee")))[#text(size: 10pt)[#item.days]],
        table.cell(stroke: (bottom: 0.3pt + rgb("#eeeeee")))[#text(size: 10pt)[#item.rate]],
        table.cell(stroke: (bottom: 0.3pt + rgb("#eeeeee")))[#text(size: 10pt)[#item.amount]],
        table.cell(stroke: (bottom: 0.3pt + rgb("#eeeeee")))[#text(size: 10pt)[#item.tax_rate]],
        table.cell(stroke: (bottom: 0.3pt + rgb("#eeeeee")))[#text(size: 10pt)[#item.tax_amount]],
      )
    },
  )
} else {
  table(
    columns: (1fr, auto, auto, auto),
    align: (left, right, right, right),
    stroke: none,
    inset: (x: 4pt, y: 12pt),
    table.header(
      text(weight: "medium", size: 8pt, fill: muted, tracking: 1pt)[DESCRIPTION],
      text(weight: "medium", size: 8pt, fill: muted, tracking: 1pt)[DAYS],
      text(weight: "medium", size: 8pt, fill: muted, tracking: 1pt)[RATE],
      text(weight: "medium", size: 8pt, fill: muted, tracking: 1pt)[AMOUNT],
    ),
    table.hline(stroke: 0.3pt + rgb("#dddddd")),
    ..for item in data.invoice.line_items {
      (
        table.cell(stroke: (bottom: 0.3pt + rgb("#eeeeee")))[#text(size: 10pt)[#item.description]],
        table.cell(stroke: (bottom: 0.3pt + rgb("#eeeeee")))[#text(size: 10pt)[#item.days]],
        table.cell(stroke: (bottom: 0.3pt + rgb("#eeeeee")))[#text(size: 10pt)[#item.rate]],
        table.cell(stroke: (bottom: 0.3pt + rgb("#eeeeee")))[#text(size: 10pt)[#item.amount]],
      )
    },
  )
}

#v(0.8cm)

// --- Totals: plain right-aligned, total underlined in accent ---
#align(right)[
  #if data.invoice.has_tax {
    table(
      columns: (auto, auto),
      align: (left, right),
      stroke: none,
      inset: (x: 10pt, y: 4pt),
      text(fill: muted)[Subtotal], text()[#data.invoice.currency #data.invoice.subtotal],
      text(fill: muted)[Tax], text()[#data.invoice.currency #data.invoice.tax_total],
    )
    v(0.2cm)
  }
  box[
    #text(16pt, weight: "bold")[#data.invoice.currency #data.invoice.total]
    #v(0.1cm)
    #line(length: 100%, stroke: 2pt + accent)
    #v(0.1cm)
    #text(7pt, fill: muted, tracking: 1pt)[TOTAL DUE]
  ]
]

#v(1.5cm)

// --- Payment details: inline single-line format ---
#text(7pt, fill: muted, tracking: 1pt)[PAYMENT]
#v(0.2cm)
#set text(size: 9pt, fill: rgb("#555555"))
#for method in data.payment {
  block(below: 6pt)[
    #if "label" in method [*#method.label* · ]IBAN #method.iban · BIC #method.bic_swift
  ]
}

// --- Footer ---
#v(1fr)
#line(length: 100%, stroke: 0.3pt + accent)
#v(0.3cm)
#align(center)[
  #set text(size: 8pt, fill: muted)
  #if "footer_text" in data.branding and data.branding.footer_text != "" {
    data.branding.footer_text
  } else {
    [Thank you for the opportunity to work together. · #data.sender.name · #data.sender.email]
  }
]
