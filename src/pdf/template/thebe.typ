// Invoice template (Thebe: Compact receipt style) -- expects `data` variable to be defined

#set page(
  paper: "a4",
  margin: (top: 1.5cm, bottom: 1.5cm, left: 1.5cm, right: 1.5cm),
)

#set text(font: data.branding.font, size: 9pt)

#let accent = rgb(data.branding.accent_color)
#let muted = rgb("#777777")

// --- Narrow centered column on A4 (receipt-style) ---
#align(center)[
  #box(width: 11cm)[

    // --- Dashed top rule ---
    #line(length: 100%, stroke: (paint: accent, thickness: 1pt, dash: "dashed"))
    #v(0.4cm)

    // --- Logo or centered sender name ---
    #align(center)[
      #if "logo_file" in data.branding {
        image(data.branding.logo_file, height: 1.2cm)
        v(0.3cm)
      }
      #text(14pt, weight: "bold", tracking: 2pt)[#upper[Invoice]]
      #v(0.15cm)
      #text(11pt, font: ("Courier New", "Courier", "monospace"))[\##data.invoice.number]
      #v(0.15cm)
      #set text(size: 8pt, fill: muted)
      #data.invoice.date  ·  due #data.invoice.due_date
    ]

    #v(0.4cm)
    #line(length: 100%, stroke: (paint: muted, thickness: 0.5pt, dash: "dotted"))
    #v(0.3cm)

    // --- Sender (centered) ---
    #align(center)[
      #set text(size: 9pt)
      *#data.sender.name* \
      #set text(size: 8pt, fill: muted)
      #for ln in data.sender.address [
        #ln \
      ]
      #data.sender.email
    ]

    #v(0.3cm)
    #line(length: 100%, stroke: (paint: muted, thickness: 0.5pt, dash: "dotted"))
    #v(0.3cm)

    // --- Recipient (centered) ---
    #align(center)[
      #text(7pt, fill: muted, tracking: 1.5pt)[#upper[Billed To]]
      #v(0.1cm)
      #text(9pt, weight: "bold")[#data.recipient.name] \
      #set text(size: 8pt, fill: muted)
      #for ln in data.recipient.address [
        #ln \
      ]
      #if "company_id" in data.recipient [
        Co. ID #data.recipient.company_id \
      ]
      #if "vat_number" in data.recipient [
        VAT #data.recipient.vat_number
      ]
    ]

    #v(0.4cm)
    #line(length: 100%, stroke: (paint: muted, thickness: 0.5pt, dash: "dotted"))
    #v(0.3cm)

    // --- Line items as receipt rows (left: description + days, right: amount) ---
    #text(7pt, fill: muted, tracking: 1.5pt)[#upper[Items]]
    #v(0.2cm)

    #set text(size: 9pt)
    #for item in data.invoice.line_items {
      grid(
        columns: (1fr, auto),
        align: (left, right),
        row-gutter: 3pt,
        [#item.description],
        text(font: ("Courier New", "Courier", "monospace"))[#data.invoice.currency #item.amount],
      )
      grid(
        columns: (1fr, auto),
        align: (left, right),
        row-gutter: 3pt,
        [#text(size: 8pt, fill: muted)[#item.days d × #item.rate · #data.invoice.period]],
        [],
      )
      v(0.15cm)
    }

    #v(0.2cm)
    #line(length: 100%, stroke: (paint: muted, thickness: 0.5pt, dash: "dotted"))
    #v(0.3cm)

    // --- Totals (centered column, mono numbers) ---
    #set text(font: ("Courier New", "Courier", "monospace"), size: 9pt)
    #if data.invoice.has_tax {
      grid(
        columns: (1fr, auto),
        align: (left, right),
        row-gutter: 4pt,
        text(font: data.branding.font)[Subtotal], [#data.invoice.currency #data.invoice.subtotal],
        text(font: data.branding.font)[Tax], [#data.invoice.currency #data.invoice.tax_total],
      )
      v(0.2cm)
    }
    #line(length: 100%, stroke: 1pt + accent)
    #v(0.15cm)
    #grid(
      columns: (1fr, auto),
      align: (left, right),
      text(font: data.branding.font, size: 11pt, weight: "bold", fill: accent)[TOTAL],
      text(size: 11pt, weight: "bold", fill: accent)[#data.invoice.currency #data.invoice.total],
    )
    #v(0.15cm)
    #line(length: 100%, stroke: 1pt + accent)

    #v(0.5cm)
    #set text(font: data.branding.font)

    // --- Payment details ---
    #align(center)[
      #text(7pt, fill: muted, tracking: 1.5pt)[#upper[Payment]]
      #v(0.15cm)
      #set text(size: 8pt)
      #for method in data.payment {
        block(below: 5pt)[
          *#method.label* \
          #text(font: ("Courier New", "Courier", "monospace"), size: 8pt)[#method.iban] \
          #text(size: 7pt, fill: muted)[BIC #method.bic_swift]
        ]
      }
    ]

    #v(0.4cm)
    #line(length: 100%, stroke: (paint: muted, thickness: 0.5pt, dash: "dotted"))
    #v(0.3cm)

    // --- Footer (centered) ---
    #align(center)[
      #set text(size: 7pt, fill: muted)
      #if "footer_text" in data.branding and data.branding.footer_text != "" {
        data.branding.footer_text
      } else {
        [thank you · #data.sender.name]
      }
    ]

    #v(0.4cm)
    #line(length: 100%, stroke: (paint: accent, thickness: 1pt, dash: "dashed"))
  ]
]
