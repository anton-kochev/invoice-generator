// Invoice template (Io: Bilingual UA/EN refined card) -- expects `data` variable to be defined.
//
// Expected `data.*` fields (in addition to the standard set used by other templates):
//   data.sender.name_ua            (string)
//   data.sender.address_ua         (array of strings)
//   data.recipient.address_ua      (array of strings)
//   data.invoice.description       (string, EN)
//   data.invoice.description_ua    (string)
//   data.invoice.currency_name_ua  (string, e.g. "Долар США")
//   data.invoice.amount_in_words   (string, EN)
//   data.invoice.amount_in_words_ua(string)
//   data.invoice.line_items[i].description_ua  (string)
//   data.payment.first() carries:
//     beneficiary, account_number, beneficiary_bank,
//     iban, bic_swift, correspondent_bank, correspondent_swift, correspondent_account

// ─── Currency symbol (falls back to ISO code + space) ───
#let currency-sym = (
  "USD": "$",
  "EUR": "€",
  "UAH": "₴",
).at(data.invoice.currency, default: data.invoice.currency + " ")

// ─── Palette ───
#let accent       = rgb(data.branding.accent_color)
#let accent-ink   = rgb("#fbf6ec")
#let ink          = rgb("#1a1d24")
#let ink-soft     = rgb("#4a4d56")
#let ink-muted    = rgb("#8a8e98")
#let rule         = rgb("#d9d4c7")
#let rule-soft    = rgb("#e8e5dd")
#let surface-bg   = rgb("#faf8f2")
#let surface-deep = rgb("#f3f0e6")

// ─── Page setup (zero margin so the accent bar goes edge-to-edge) ───
#set page(paper: "a4", margin: 0cm)
#set text(font: data.branding.font, size: 9.5pt, fill: ink)
#set block(above: 0pt, below: 0pt)

// ─── Font stacks ───
#let serif-font = ("Iowan Old Style", "Source Serif 4", "Source Serif Pro", "Georgia", "Times New Roman", "DejaVu Serif")
#let mono-font  = ("JetBrains Mono", "SF Mono", "Menlo", "Courier New", "DejaVu Sans Mono", "Liberation Mono")

// ─── Side padding for body sections ───
#let pad-x = 1.8cm

// ─── Reusable helpers ───
#let caps-label(s) = text(
  size: 6.5pt,
  weight: "semibold",
  tracking: 0.16em,
  fill: ink-muted,
)[#upper(s)]

#let bilabel(en, ua) = {
  caps-label(en)
  text(size: 7pt, fill: ink-muted)[~/~]
  text(size: 7pt, weight: "regular", fill: ink-muted)[#ua]
}

// ═══ HEADER: accent bar with bilingual title ═══
#block(
  fill: accent,
  width: 100%,
  inset: (x: pad-x, y: 0.4cm),
)[
  #align(center)[
    #text(font: serif-font, size: 19pt, weight: "semibold", fill: accent-ink)[
      Інвойс
      #text(weight: "regular", fill: accent-ink.transparentize(45%))[~/~]
      Invoice
    ]
    #v(0.15cm)
    #text(size: 8.5pt, tracking: 0.08em, fill: accent-ink.transparentize(30%))[
      #data.invoice.number · #data.invoice.date
    ]
  ]
]

// ═══ INFO GRID: bilingual cells (UA tinted left, EN white right) ═══
#grid(
  columns: (1fr, 1fr),
  inset: (x: pad-x, y: 0.28cm),
  fill: (x, _y) => if calc.rem(x, 2) == 0 { surface-bg } else { white },
  stroke: (x, _y) => (
    bottom: 0.4pt + rule-soft,
    right: if calc.rem(x, 2) == 0 { 0.4pt + rule-soft } else { none },
  ),

  // ── Supplier / Виконавець ──
  [
    #caps-label("Виконавець")
    #v(0.2cm)
    #text(font: serif-font, size: 11pt, weight: "medium")[#data.sender.name_ua]
    #v(0.15cm)
    #set text(size: 9pt, fill: ink-soft)
    #for ln in data.sender.address_ua [#ln \ ]
  ],
  [
    #caps-label("Supplier")
    #v(0.2cm)
    #text(font: serif-font, size: 11pt, weight: "medium")[#data.sender.name]
    #v(0.15cm)
    #set text(size: 9pt, fill: ink-soft)
    #for ln in data.sender.address [#ln \ ]
  ],

  // ── Customer / Замовник ──
  [
    #caps-label("Замовник")
    #v(0.2cm)
    #text(font: serif-font, size: 11pt, weight: "medium")[#data.recipient.name]
    #v(0.15cm)
    #set text(size: 9pt, fill: ink-soft)
    #for ln in data.recipient.address_ua [#ln \ ]
  ],
  [
    #caps-label("Customer")
    #v(0.2cm)
    #text(font: serif-font, size: 11pt, weight: "medium")[#data.recipient.name]
    #v(0.15cm)
    #set text(size: 9pt, fill: ink-soft)
    #for ln in data.recipient.address [#ln \ ]
  ],

  // ── Description / Опис ──
  [
    #caps-label("Опис")
    #v(0.18cm)
    #data.invoice.description_ua
  ],
  [
    #caps-label("Description")
    #v(0.18cm)
    #data.invoice.description
  ],

  // ── Currency / Валюта ──
  [
    #caps-label("Валюта")
    #v(0.18cm)
    #data.invoice.currency_name_ua
  ],
  [
    #caps-label("Currency")
    #v(0.18cm)
    #data.invoice.currency
  ],
)

// ═══ BANK INFORMATION (white block, centered ledger) ═══
#let pay = data.payment.first()

#block(
  fill: white,
  width: 100%,
  inset: (x: pad-x, y: 0.55cm),
  stroke: (bottom: 0.4pt + rule-soft),
)[
  #align(center)[#bilabel("Bank information", "Банківські реквізити")]
  #v(0.35cm)

  #table(
    columns: (auto, 1fr),
    stroke: none,
    inset: (x: 0pt, y: 3pt),
    column-gutter: 1.5cm,

    caps-label("Beneficiary"),    text(size: 9.5pt)[#pay.beneficiary],
    caps-label("Bank"),           text(size: 9.5pt)[#pay.beneficiary_bank],
    caps-label("Account #"),      text(font: mono-font, size: 9pt)[#pay.account_number],
    caps-label("IBAN"),           text(font: mono-font, size: 9pt)[#pay.iban],
    caps-label("SWIFT"),          text(font: mono-font, size: 9pt)[#pay.bic_swift],

    table.cell(colspan: 2, inset: (top: 10pt, bottom: 6pt))[
      #align(center)[#bilabel("Correspondent", "Кореспондент")]
    ],

    caps-label("Bank"),           text(size: 9.5pt)[#pay.correspondent_bank],
    caps-label("Account #"),      text(font: mono-font, size: 9pt)[#pay.correspondent_account],
    caps-label("SWIFT"),          text(font: mono-font, size: 9pt)[#pay.correspondent_swift],
  )
]

// ═══ LINE ITEMS (4-col grid + Total to pay climax) ═══
#block(
  width: 100%,
  inset: (x: pad-x, top: 0.4cm, bottom: 0.25cm),
)[
  #table(
    columns: (auto, 1fr, auto, auto),
    align: (center + horizon, left + horizon, right + horizon, right + horizon),
    stroke: none,
    inset: (x: 6pt, y: 5pt),
    column-gutter: 0.4cm,

    // Header row (with hairline underneath via cell strokes)
    table.cell(stroke: (bottom: 0.4pt + rule-soft))[#caps-label("№")],
    table.cell(stroke: (bottom: 0.4pt + rule-soft))[
      #caps-label("Description")
      #v(2pt)
      #text(size: 7.5pt, fill: ink-muted)[Опис]
    ],
    table.cell(stroke: (bottom: 0.4pt + rule-soft))[
      #caps-label("Price")
      #v(2pt)
      #text(size: 7.5pt, fill: ink-muted)[Ціна]
    ],
    table.cell(stroke: (bottom: 0.4pt + rule-soft))[
      #caps-label("Amount")
      #v(2pt)
      #text(size: 7.5pt, fill: ink-muted)[Сума]
    ],

    // Body rows
    ..for (i, item) in data.invoice.line_items.enumerate() {
      (
        text(fill: ink-muted)[#(i + 1)],
        [
          #text(weight: "medium")[#item.description]
          #v(2pt)
          #text(size: 8.5pt, fill: ink-muted)[#item.description_ua]
        ],
        text(fill: ink-muted)[#item.rate],
        text(weight: "medium")[#item.amount],
      )
    },

    // Total to pay row (col 1 empty; col 2 label; col 3 amount-in-words; col 4 amount)
    table.cell(inset: (top: 8pt, bottom: 2pt))[],
    table.cell(inset: (top: 8pt, bottom: 2pt), align: left + horizon)[
      #text(size: 9.5pt, weight: "semibold")[Total to pay] \
      #text(size: 8.5pt, fill: ink-muted)[Усього до сплати]
    ],
    table.cell(inset: (top: 8pt, bottom: 2pt), align: right + horizon)[
      #text(size: 9.5pt, fill: ink-soft)[#data.invoice.amount_in_words] \
      #text(size: 8.5pt, fill: ink-muted)[#data.invoice.amount_in_words_ua]
    ],
    table.cell(inset: (top: 8pt, bottom: 2pt), align: right + horizon)[
      #text(size: 11pt, weight: "semibold")[#currency-sym#data.invoice.total]
    ],
  )
]

// ═══ DISCLAIMER (warm tint block) ═══
#block(
  fill: surface-deep,
  width: 100%,
  inset: (x: pad-x, top: 0.35cm, bottom: 0.4cm),
)[
  #text(weight: "semibold", size: 9pt)[
    All charges of correspondent banks are at the Customer's expenses. / Усі комісії банків-кореспондентів сплачує замовник.
  ]
  #v(0.22cm)
  #text(size: 8.5pt, fill: ink-soft)[
    Payment according to this invoice is the confirmation of performed
    works, delivered services, and final mutual installments between
    Parties without any additional documents. Parties acknowledge they
    have no claims to each other.
  ]
  #v(0.22cm)
  #text(size: 8.5pt, fill: ink-soft)[
    Оплата згідно цього Інвойсу одночасно є підтвердженням виконаних
    робіт, наданих послуг, кінцевих розрахунків між Сторонами і того,
    що Сторони не мають взаємних претензій, і не вимагає підписання
    додаткових документів.
  ]
]

// ═══ SIGNATURE (white block, role label + signature line) ═══
#block(
  fill: white,
  width: 100%,
  inset: (x: pad-x, y: 0.8cm),
  stroke: (top: 0.4pt + rule-soft),
)[
  #grid(
    columns: (1fr, 2fr),
    column-gutter: 2cm,
    align: bottom,
    [
      #text(size: 9pt, fill: ink-soft)[Supplier] \
      #text(size: 8.5pt, fill: ink-muted)[Виконавець]
    ],
    [
      #text(font: serif-font, size: 11pt)[#data.sender.name] \
      #text(size: 9pt, fill: ink-soft)[#data.sender.name_ua]
      #v(0.2cm)
      #line(length: 100%, stroke: 0.5pt + ink)
    ],
  )
]
