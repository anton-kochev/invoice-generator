// Invoice template -- expects `data` variable to be defined
// data structure: { sender, recipient, invoice, payment }

#set page(
  paper: "a4",
  margin: (top: 2.5cm, bottom: 2cm, left: 2.5cm, right: 2.5cm),
)

#set text(font: data.branding.font, size: 10pt)

#let accent = rgb(data.branding.accent_color)

// --- Header ---
#if "logo_file" in data.branding {
  grid(
    columns: (auto, 1fr),
    gutter: 1cm,
    image(data.branding.logo_file, height: 1.5cm),
    align(right)[
      #text(24pt, weight: "bold", fill: accent)[INVOICE]
    ],
  )
} else {
  align(right)[
    #text(24pt, weight: "bold", fill: accent)[INVOICE]
  ]
}

#v(0.3cm)

#align(right)[
  #set text(size: 9pt, fill: rgb("#555555"))
  Invoice \##data.invoice.number \
  Date: #data.invoice.date \
  Due: #data.invoice.due_date
]

#v(0.8cm)

// --- Parties: FROM and TO side by side ---
#grid(
  columns: (1fr, 1fr),
  gutter: 1cm,
  [
    #text(9pt, weight: "bold", fill: accent)[FROM]
    #v(0.2cm)
    #set text(size: 9pt)
    *#data.sender.name* \
    #for line in data.sender.address [
      #line \
    ]
    #data.sender.email
  ],
  [
    #text(9pt, weight: "bold", fill: accent)[TO]
    #v(0.2cm)
    #set text(size: 9pt)
    *#data.recipient.name* \
    #for line in data.recipient.address [
      #line \
    ]
    #if "company_id" in data.recipient [
      Company ID: #data.recipient.company_id \
    ]
    #if "vat_number" in data.recipient [
      VAT: #data.recipient.vat_number
    ]
  ],
)

#v(0.8cm)

// --- Line Items Table ---
#line(length: 100%, stroke: 0.5pt + accent)
#v(0.2cm)

#if data.invoice.has_tax {
  table(
    columns: (1fr, auto, auto, auto, auto, auto, auto),
    align: (left, center, right, right, right, right, right),
    stroke: none,
    inset: (x: 8pt, y: 6pt),
    fill: (_, y) => if calc.odd(y) and y > 0 { rgb("#f5f5f5") },
    table.header(
      text(weight: "bold", size: 9pt)[Description],
      text(weight: "bold", size: 9pt)[Period],
      text(weight: "bold", size: 9pt)[Days],
      text(weight: "bold", size: 9pt, fill: rgb("#555"))[Rate (#data.invoice.currency/MD)],
      text(weight: "bold", size: 9pt)[Amount (#data.invoice.currency)],
      text(weight: "bold", size: 9pt)[Tax (%)],
      text(weight: "bold", size: 9pt)[Tax Amt (#data.invoice.currency)],
    ),
    ..for item in data.invoice.line_items {
      (
        text(size: 9pt)[#item.description],
        text(size: 9pt)[#data.invoice.period],
        text(size: 9pt)[#item.days],
        text(size: 9pt)[#item.rate],
        text(size: 9pt, weight: "medium")[#item.amount],
        text(size: 9pt)[#item.tax_rate],
        text(size: 9pt, weight: "medium")[#item.tax_amount],
      )
    },
  )
} else {
  table(
    columns: (1fr, auto, auto, auto, auto),
    align: (left, center, right, right, right),
    stroke: none,
    inset: (x: 8pt, y: 6pt),
    fill: (_, y) => if calc.odd(y) and y > 0 { rgb("#f5f5f5") },
    table.header(
      text(weight: "bold", size: 9pt)[Description],
      text(weight: "bold", size: 9pt)[Period],
      text(weight: "bold", size: 9pt)[Days],
      text(weight: "bold", size: 9pt, fill: rgb("#555"))[Rate (#data.invoice.currency/MD)],
      text(weight: "bold", size: 9pt)[Amount (#data.invoice.currency)],
    ),
    ..for item in data.invoice.line_items {
      (
        text(size: 9pt)[#item.description],
        text(size: 9pt)[#data.invoice.period],
        text(size: 9pt)[#item.days],
        text(size: 9pt)[#item.rate],
        text(size: 9pt, weight: "medium")[#item.amount],
      )
    },
  )
}

#v(0.1cm)
#line(length: 100%, stroke: 0.5pt + accent)

// --- Total ---
#if data.invoice.has_tax {
  align(right)[
    #v(0.3cm)
    #text(10pt)[SUBTOTAL #data.invoice.currency #data.invoice.subtotal]
    #v(0.1cm)
    #text(10pt)[TAX #data.invoice.currency #data.invoice.tax_total]
    #v(0.1cm)
    #text(12pt, weight: "bold")[TOTAL #data.invoice.currency #data.invoice.total]
  ]
} else {
  align(right)[
    #v(0.3cm)
    #text(12pt, weight: "bold")[TOTAL #data.invoice.currency #data.invoice.total]
  ]
}

#v(1cm)

// --- Payment Details ---
#text(10pt, weight: "bold", fill: accent)[Payment Details]
#v(0.3cm)

#for method in data.payment {
  block(inset: (left: 0pt, bottom: 8pt))[
    #set text(size: 9pt)
    *#method.label* \
    IBAN: #method.iban \
    BIC/SWIFT: #method.bic_swift
  ]
}

#v(1fr)

// --- Footer ---
#let footer_content = if "footer_text" in data.branding and data.branding.footer_text != "" {
  data.branding.footer_text
} else {
  [Thank you for the opportunity to work together. \
   #data.sender.name · #data.sender.email]
}
#line(length: 100%, stroke: 0.3pt + rgb("#cccccc"))
#v(0.3cm)
#align(center)[
  #set text(size: 8pt, fill: rgb("#888888"))
  #footer_content
]
