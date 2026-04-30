// Invoice template (Callisto: Corporate formal with sidebar) -- expects `data` variable to be defined

#set page(
  paper: "a4",
  margin: (top: 0cm, bottom: 0cm, left: 0cm, right: 0cm),
)

#set text(font: data.branding.font, size: 10pt)

#let accent = rgb(data.branding.accent_color)
#let sidebar-fill = rgb("#f4f4f4")
#let border = 0.5pt + rgb("#c8c8c8")

// --- Two-column page layout: sidebar (35%) + main column (65%) ---
#grid(
  columns: (35%, 65%),
  rows: (100%,),

  // ======== SIDEBAR ========
  rect(
    fill: sidebar-fill,
    width: 100%,
    height: 100%,
    stroke: (right: 1pt + accent),
    inset: (x: 1.2cm, y: 2cm),
  )[
    #if "logo_file" in data.branding {
      image(data.branding.logo_file, height: 1.4cm)
      v(0.6cm)
    }

    #text(8pt, weight: "bold", fill: accent, tracking: 1pt)[SENDER]
    #v(0.1cm)
    #line(length: 100%, stroke: border)
    #v(0.2cm)
    #set text(size: 9pt)
    *#data.sender.name* \
    #for ln in data.sender.address [
      #ln \
    ]
    #data.sender.email

    #v(0.8cm)

    #text(8pt, weight: "bold", fill: accent, tracking: 1pt)[PAYMENT]
    #v(0.1cm)
    #line(length: 100%, stroke: border)
    #v(0.2cm)
    #for method in data.payment {
      block(below: 10pt)[
        #set text(size: 9pt)
        *#method.label* \
        IBAN: #method.iban \
        BIC: #method.bic_swift
      ]
    }

    // --- Footer pinned to sidebar bottom ---
    #v(1fr)
    #line(length: 100%, stroke: border)
    #v(0.2cm)
    #set text(size: 7pt, fill: rgb("#666666"))
    #if "footer_text" in data.branding and data.branding.footer_text != "" {
      data.branding.footer_text
    } else {
      [Thank you for the opportunity \
       to work together. \
       #data.sender.name]
    }
  ],

  // ======== MAIN COLUMN ========
  block(inset: (x: 1.5cm, y: 2cm), width: 100%)[
    // --- Title + meta ---
    #grid(
      columns: (1fr, auto),
      align: (left + horizon, right + horizon),
      [
        #text(28pt, weight: "bold", fill: accent)[INVOICE]
        #v(0.1cm)
        #text(9pt, fill: rgb("#666666"))[No. #data.invoice.number]
      ],
      [
        #set text(size: 9pt)
        #table(
          columns: (auto, auto),
          align: (left, right),
          stroke: none,
          inset: (x: 4pt, y: 2pt),
          text(fill: rgb("#666666"))[Issued], text(weight: "bold")[#data.invoice.date],
          text(fill: rgb("#666666"))[Due],    text(weight: "bold")[#data.invoice.due_date],
          text(fill: rgb("#666666"))[Period], text(weight: "bold")[#data.invoice.period],
        )
      ],
    )

    #v(0.4cm)
    #line(length: 100%, stroke: 1pt + accent)
    #v(0.6cm)

    // --- Recipient ---
    #text(8pt, weight: "bold", fill: accent, tracking: 1pt)[BILL TO]
    #v(0.2cm)
    #set text(size: 10pt)
    *#data.recipient.name* \
    #set text(size: 9pt)
    #for ln in data.recipient.address [
      #ln \
    ]
    #if "company_id" in data.recipient [
      Company ID: #data.recipient.company_id \
    ]
    #if "vat_number" in data.recipient [
      VAT: #data.recipient.vat_number
    ]

    #v(0.7cm)

    // --- Line Items Table: all borders (formal/structured) ---
    #if data.invoice.has_tax {
      table(
        columns: (1fr, auto, auto, auto, auto, auto, auto),
        align: (left, center, right, right, right, right, right),
        stroke: border,
        inset: (x: 6pt, y: 6pt),
        fill: (_, y) => if y == 0 { accent.lighten(88%) },
        table.header(
          text(weight: "bold", size: 8pt)[Description],
          text(weight: "bold", size: 8pt)[Period],
          text(weight: "bold", size: 8pt)[Days],
          text(weight: "bold", size: 8pt)[Rate],
          text(weight: "bold", size: 8pt)[Amount],
          text(weight: "bold", size: 8pt)[Tax %],
          text(weight: "bold", size: 8pt)[Tax Amt],
        ),
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
        stroke: border,
        inset: (x: 6pt, y: 6pt),
        fill: (_, y) => if y == 0 { accent.lighten(88%) },
        table.header(
          text(weight: "bold", size: 8pt)[Description],
          text(weight: "bold", size: 8pt)[Period],
          text(weight: "bold", size: 8pt)[Days],
          text(weight: "bold", size: 8pt)[Rate (#data.invoice.currency/MD)],
          text(weight: "bold", size: 8pt)[Amount (#data.invoice.currency)],
        ),
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

    #v(0.6cm)

    // --- Totals: right-aligned bordered box ---
    #align(right)[
      #rect(stroke: border, inset: 10pt)[
        #set text(size: 9pt)
        #if data.invoice.has_tax {
          table(
            columns: (auto, auto),
            align: (left, right),
            stroke: none,
            inset: (x: 6pt, y: 3pt),
            [Subtotal], [#data.invoice.currency #data.invoice.subtotal],
            [Tax],      [#data.invoice.currency #data.invoice.tax_total],
            table.hline(stroke: border),
            text(weight: "bold", size: 12pt, fill: accent)[TOTAL], text(weight: "bold", size: 12pt, fill: accent)[#data.invoice.currency #data.invoice.total],
          )
        } else {
          text(12pt, weight: "bold", fill: accent)[TOTAL  #data.invoice.currency #data.invoice.total]
        }
      ]
    ]
  ],
)
