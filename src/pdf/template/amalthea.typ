// Invoice template (Amalthea: Bold & brand-forward) -- expects `data` variable to be defined

#set page(
  paper: "a4",
  margin: (top: 0cm, bottom: 0cm, left: 0cm, right: 0cm),
)

#set text(font: data.branding.font, size: 11pt)

#let accent = rgb(data.branding.accent_color)

// --- Full-width accent banner containing header + sender ---
#rect(fill: accent, width: 100%, inset: (x: 2cm, y: 1.4cm))[
  #grid(
    columns: (1fr, auto),
    align: (left + top, right + top),
    gutter: 1cm,
    [
      #text(34pt, weight: "bold", fill: white)[INVOICE]
      #v(0.2cm)
      #set text(size: 10pt, fill: white)
      *\##data.invoice.number*  ·  #data.invoice.date  ·  due #data.invoice.due_date
      #v(0.5cm)
      #text(9pt, weight: "bold", fill: white.transparentize(20%))[FROM]
      #v(0.1cm)
      #set text(size: 10pt, fill: white)
      *#data.sender.name* \
      #for ln in data.sender.address [
        #ln \
      ]
      #data.sender.email
    ],
    [
      #if "logo_file" in data.branding {
        box(fill: white, inset: 8pt, radius: 2pt)[
          #image(data.branding.logo_file, height: 1.8cm)
        ]
      }
    ],
  )
]

// --- Body (inside padded container) ---
#block(inset: (x: 2cm, y: 1cm))[

  // --- Recipient in a bordered accent card ---
  #rect(
    stroke: 1.5pt + accent,
    inset: 14pt,
    width: 100%,
    radius: 2pt,
  )[
    #text(9pt, weight: "bold", fill: accent)[BILL TO]
    #v(0.2cm)
    #set text(size: 11pt)
    *#data.recipient.name* \
    #set text(size: 10pt)
    #for ln in data.recipient.address [
      #ln \
    ]
    #if "company_id" in data.recipient [
      Company ID: #data.recipient.company_id \
    ]
    #if "vat_number" in data.recipient [
      VAT: #data.recipient.vat_number
    ]
  ]

  #v(0.8cm)

  // --- Line Items Table: accent header row, alternating fills ---
  #let row-fill = (_, y) => {
    if y == 0 { accent }
    else if calc.odd(y) { accent.lighten(92%) }
    else { white }
  }

  #if data.invoice.has_tax {
    table(
      columns: (1fr, auto, auto, auto, auto, auto, auto),
      align: (left, center, right, right, right, right, right),
      stroke: none,
      inset: (x: 10pt, y: 8pt),
      fill: row-fill,
      table.header(
        text(weight: "bold", size: 9pt, fill: white)[Description],
        text(weight: "bold", size: 9pt, fill: white)[Period],
        text(weight: "bold", size: 9pt, fill: white)[Days],
        text(weight: "bold", size: 9pt, fill: white)[Rate (#data.invoice.currency/MD)],
        text(weight: "bold", size: 9pt, fill: white)[Amount (#data.invoice.currency)],
        text(weight: "bold", size: 9pt, fill: white)[Tax (%)],
        text(weight: "bold", size: 9pt, fill: white)[Tax Amt (#data.invoice.currency)],
      ),
      ..for item in data.invoice.line_items {
        (
          text(size: 10pt)[#item.description],
          text(size: 10pt)[#data.invoice.period],
          text(size: 10pt)[#item.days],
          text(size: 10pt)[#item.rate],
          text(size: 10pt)[#item.amount],
          text(size: 10pt)[#item.tax_rate],
          text(size: 10pt)[#item.tax_amount],
        )
      },
    )
  } else {
    table(
      columns: (1fr, auto, auto, auto, auto),
      align: (left, center, right, right, right),
      stroke: none,
      inset: (x: 10pt, y: 8pt),
      fill: row-fill,
      table.header(
        text(weight: "bold", size: 9pt, fill: white)[Description],
        text(weight: "bold", size: 9pt, fill: white)[Period],
        text(weight: "bold", size: 9pt, fill: white)[Days],
        text(weight: "bold", size: 9pt, fill: white)[Rate (#data.invoice.currency/MD)],
        text(weight: "bold", size: 9pt, fill: white)[Amount (#data.invoice.currency)],
      ),
      ..for item in data.invoice.line_items {
        (
          text(size: 10pt)[#item.description],
          text(size: 10pt)[#data.invoice.period],
          text(size: 10pt)[#item.days],
          text(size: 10pt)[#item.rate],
          text(size: 10pt)[#item.amount],
        )
      },
    )
  }

  #v(0.8cm)

  // --- Totals: right-floated accent card with breakdown inside ---
  #align(right)[
    #rect(fill: accent, inset: 14pt, radius: 2pt)[
      #set text(fill: white)
      #if data.invoice.has_tax {
        table(
          columns: (auto, auto),
          align: (left, right),
          stroke: none,
          inset: (x: 8pt, y: 3pt),
          [Subtotal], [#data.invoice.currency #data.invoice.subtotal],
          [Tax], [#data.invoice.currency #data.invoice.tax_total],
          text(weight: "bold", size: 14pt)[TOTAL], text(weight: "bold", size: 14pt)[#data.invoice.currency #data.invoice.total],
        )
      } else {
        text(14pt, weight: "bold")[TOTAL  #data.invoice.currency #data.invoice.total]
      }
    ]
  ]
]

// --- Bottom full-width accent strip with payment details ---
#v(1fr)

#rect(fill: accent, width: 100%, inset: (x: 2cm, y: 1cm))[
  #text(9pt, weight: "bold", fill: white.transparentize(20%))[PAYMENT DETAILS]
  #v(0.2cm)
  #set text(size: 10pt, fill: white)
  #grid(
    columns: (1fr,) * calc.min(data.payment.len(), 3),
    gutter: 1cm,
    ..for method in data.payment {
      ([
        #if "label" in method [
          *#method.label* \
        ]
        IBAN: #method.iban \
        BIC/SWIFT: #method.bic_swift
      ],)
    },
  )

  #v(0.5cm)
  #line(length: 100%, stroke: 0.5pt + white.transparentize(50%))
  #v(0.3cm)
  #align(center)[
    #set text(size: 8pt, fill: white.transparentize(20%))
    #if "footer_text" in data.branding and data.branding.footer_text != "" {
      data.branding.footer_text
    } else {
      [Thank you for the opportunity to work together. · #data.sender.name · #data.sender.email]
    }
  ]
]
