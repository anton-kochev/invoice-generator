// Invoice template (Metis: Editorial masthead) -- expects `data` variable to be defined

#set page(
  paper: "a4",
  margin: (top: 2cm, bottom: 2cm, left: 2.2cm, right: 2.2cm),
)

#set text(font: data.branding.font, size: 10pt)

#let accent = rgb(data.branding.accent_color)
#let rule-color = rgb("#222222")

// --- Masthead: sender "publisher" left, INVOICE headline right ---
#line(length: 100%, stroke: 1.5pt + accent)
#v(0.15cm)
#line(length: 100%, stroke: 0.3pt + accent)
#v(0.3cm)

#grid(
  columns: (1fr, auto),
  align: (left + horizon, right + horizon),
  [
    #if "logo_file" in data.branding {
      image(data.branding.logo_file, height: 1.2cm)
    } else {
      text(11pt, weight: "bold", tracking: 2pt)[#upper(data.sender.name)]
    }
  ],
  [
    #text(26pt, weight: "bold", style: "italic")[Invoice]
    #h(0.2cm)
    #text(20pt, weight: "light", fill: accent)[№ #data.invoice.number]
  ],
)

#v(0.2cm)
#line(length: 100%, stroke: 0.3pt + accent)
#v(0.1cm)
#line(length: 100%, stroke: 1.5pt + accent)

#v(0.3cm)

// --- Byline: date / due / period ---
#align(center)[
  #set text(size: 8pt, tracking: 1.5pt)
  #upper[#data.invoice.date] #h(0.4cm) — #h(0.4cm) #upper[Due #data.invoice.due_date] #h(0.4cm) — #h(0.4cm) #upper[Period #data.invoice.period]
]

#v(0.9cm)

// --- 3-column grid: FROM | TO | SUMMARY ---
#grid(
  columns: (1fr, 1fr, 1fr),
  gutter: 0.8cm,
  [
    #text(7pt, weight: "bold", tracking: 1.5pt)[#upper[From]]
    #v(0.1cm)
    #line(length: 100%, stroke: 0.3pt + rule-color)
    #v(0.2cm)
    #set text(size: 9pt)
    *#data.sender.name* \
    #for ln in data.sender.address [
      #ln \
    ]
    #data.sender.email
  ],
  [
    #text(7pt, weight: "bold", tracking: 1.5pt)[#upper[Billed To]]
    #v(0.1cm)
    #line(length: 100%, stroke: 0.3pt + rule-color)
    #v(0.2cm)
    #set text(size: 9pt)
    *#data.recipient.name* \
    #for ln in data.recipient.address [
      #ln \
    ]
    #if "company_id" in data.recipient [
      Co. ID — #data.recipient.company_id \
    ]
    #if "vat_number" in data.recipient [
      VAT — #data.recipient.vat_number
    ]
  ],
  [
    #text(7pt, weight: "bold", tracking: 1.5pt)[#upper[Summary]]
    #v(0.1cm)
    #line(length: 100%, stroke: 0.3pt + rule-color)
    #v(0.2cm)
    #set text(size: 9pt)
    #if data.invoice.has_tax [
      Subtotal — #data.invoice.currency #data.invoice.subtotal \
      Tax — #data.invoice.currency #data.invoice.tax_total \
    ]
    #text(weight: "bold", fill: accent)[Total — #data.invoice.currency #data.invoice.total]
  ],
)

#v(0.9cm)

// --- Line Items: rules only, small caps headers ---
#line(length: 100%, stroke: 0.8pt + rule-color)
#v(0.15cm)

#if data.invoice.has_tax {
  table(
    columns: (1fr, auto, auto, auto, auto, auto, auto),
    align: (left, center, right, right, right, right, right),
    stroke: none,
    inset: (x: 6pt, y: 7pt),
    table.header(
      text(weight: "bold", size: 7pt, tracking: 1pt)[#upper[Description]],
      text(weight: "bold", size: 7pt, tracking: 1pt)[#upper[Period]],
      text(weight: "bold", size: 7pt, tracking: 1pt)[#upper[Days]],
      text(weight: "bold", size: 7pt, tracking: 1pt)[#upper[Rate]],
      text(weight: "bold", size: 7pt, tracking: 1pt)[#upper[Amount]],
      text(weight: "bold", size: 7pt, tracking: 1pt)[#upper[Tax \%]],
      text(weight: "bold", size: 7pt, tracking: 1pt)[#upper[Tax Amt]],
    ),
    table.hline(stroke: 0.3pt + rule-color),
    ..for item in data.invoice.line_items {
      (
        text(size: 9pt)[#item.description],
        text(size: 9pt)[#data.invoice.period],
        text(size: 9pt)[#item.days],
        text(size: 9pt)[#item.rate],
        text(size: 9pt)[#item.amount],
        text(size: 9pt)[#item.tax_rate],
        text(size: 9pt)[#item.tax_amount],
      )
    },
  )
} else {
  table(
    columns: (1fr, auto, auto, auto, auto),
    align: (left, center, right, right, right),
    stroke: none,
    inset: (x: 6pt, y: 7pt),
    table.header(
      text(weight: "bold", size: 7pt, tracking: 1pt)[#upper[Description]],
      text(weight: "bold", size: 7pt, tracking: 1pt)[#upper[Period]],
      text(weight: "bold", size: 7pt, tracking: 1pt)[#upper[Days]],
      text(weight: "bold", size: 7pt, tracking: 1pt)[#upper[Rate (#data.invoice.currency/MD)]],
      text(weight: "bold", size: 7pt, tracking: 1pt)[#upper[Amount (#data.invoice.currency)]],
    ),
    table.hline(stroke: 0.3pt + rule-color),
    ..for item in data.invoice.line_items {
      (
        text(size: 9pt)[#item.description],
        text(size: 9pt)[#data.invoice.period],
        text(size: 9pt)[#item.days],
        text(size: 9pt)[#item.rate],
        text(size: 9pt)[#item.amount],
      )
    },
  )
}

#line(length: 100%, stroke: 0.8pt + rule-color)

#v(0.6cm)

// --- Totals: right-aligned em-dash style ---
#align(right)[
  #set text(size: 10pt)
  #if data.invoice.has_tax [
    Subtotal — #data.invoice.currency #data.invoice.subtotal \
    Tax — #data.invoice.currency #data.invoice.tax_total \
  ]
  #v(0.15cm)
  #box[
    #line(length: 6cm, stroke: 0.3pt + rule-color)
    #v(0.1cm)
    #text(13pt, weight: "bold")[Total — #data.invoice.currency #data.invoice.total]
  ]
]

#v(1fr)

// --- Payment footer: 2-column grid ---
#line(length: 100%, stroke: 0.3pt + rule-color)
#v(0.3cm)

#text(7pt, weight: "bold", tracking: 1.5pt)[#upper[Payment]]
#v(0.2cm)

#grid(
  columns: (1fr,) * calc.max(calc.min(data.payment.len(), 2), 1),
  gutter: 0.8cm,
  ..for method in data.payment {
    ([
      #set text(size: 8pt)
      *#method.label* \
      IBAN — #method.iban \
      BIC — #method.bic_swift
    ],)
  },
)

#v(0.4cm)
#line(length: 100%, stroke: 1pt + accent)
#v(0.15cm)
#align(center)[
  #set text(size: 7pt, tracking: 1pt, fill: rgb("#666666"))
  #if "footer_text" in data.branding and data.branding.footer_text != "" {
    upper(data.branding.footer_text)
  } else {
    upper[Thank you for the opportunity to work together]
  }
]
