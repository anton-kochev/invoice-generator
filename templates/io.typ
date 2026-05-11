// Invoice template (Io: Bilingual UA/EN refined card) -- expects `data` variable to be defined.
//
// All template-specific extras live on `data.sender.*` and are sourced from
// the optional `Sender.extras` mapping in the user's YAML config (carried
// through to JSON via `#[serde(flatten)]` on `SenderData.extras`). Missing
// keys gracefully fall back to a sensible standard-data default below, so
// the template still renders even when no extras are configured.
//
// Optional `data.sender.*` keys read by this template:
//   name_ua                    (string)            — falls back to data.sender.name
//   address_ua                 (array of strings)  — falls back to data.sender.address
//   recipient_address_ua       (array of strings)  — falls back to data.recipient.address
//   invoice_description        (string, EN)        — falls back to joined line-item descriptions
//   invoice_description_ua     (string)            — falls back to invoice_description fallback
//   currency_name_ua           (string)            — falls back to data.invoice.currency
//   amount_in_words            (string, EN)        — falls back to data.invoice.total
//   amount_in_words_ua         (string)            — falls back to data.invoice.total
//   beneficiary                (string)            — falls back to data.sender.name
//   account_number             (string)            — falls back to the first payment IBAN
//   beneficiary_bank           (string)            — falls back to em-dash
//   correspondent_bank         (string)            — falls back to em-dash
//   correspondent_swift        (string)            — falls back to em-dash
//   correspondent_account      (string)            — falls back to em-dash

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

// ─── Extras fallbacks (read once for clarity) ───
#let sender-name-ua       = data.sender.at("name_ua",              default: data.sender.name)
#let sender-address-ua    = data.sender.at("address_ua",           default: data.sender.address)
#let recipient-address-ua = data.sender.at("recipient_address_ua", default: data.recipient.address)
#let invoice-desc-fallback = data.invoice.line_items.map(it => it.description).join(", ")
#let invoice-desc          = data.sender.at("invoice_description",    default: invoice-desc-fallback)
#let invoice-desc-ua       = data.sender.at("invoice_description_ua", default: invoice-desc-fallback)
#let currency-name-ua      = data.sender.at("currency_name_ua",       default: data.invoice.currency)
#let amount-in-words       = data.sender.at("amount_in_words",        default: data.invoice.total)
#let amount-in-words-ua    = data.sender.at("amount_in_words_ua",     default: data.invoice.total)

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
    #text(font: serif-font, size: 11pt, weight: "medium")[#sender-name-ua]
    #v(0.15cm)
    #set text(size: 9pt, fill: ink-soft)
    #for ln in sender-address-ua [#ln \ ]
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
    #for ln in recipient-address-ua [#ln \ ]
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
    #invoice-desc-ua
  ],
  [
    #caps-label("Description")
    #v(0.18cm)
    #invoice-desc
  ],

  // ── Currency / Валюта ──
  [
    #caps-label("Валюта")
    #v(0.18cm)
    #currency-name-ua
  ],
  [
    #caps-label("Currency")
    #v(0.18cm)
    #data.invoice.currency
  ],
)

// ═══ BANK INFORMATION (white block, centered ledger) ═══
#let pay = data.payment.first()

#let beneficiary           = data.sender.at("beneficiary",           default: data.sender.name)
#let account-number        = data.sender.at("account_number",        default: pay.iban)
#let beneficiary-bank      = data.sender.at("beneficiary_bank",      default: "—")
#let correspondent-bank    = data.sender.at("correspondent_bank",    default: "—")
#let correspondent-swift   = data.sender.at("correspondent_swift",   default: "—")
#let correspondent-account = data.sender.at("correspondent_account", default: "—")

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

    caps-label("Beneficiary"),    text(size: 9.5pt)[#beneficiary],
    caps-label("Bank"),           text(size: 9.5pt)[#beneficiary-bank],
    caps-label("Account #"),      text(font: mono-font, size: 9pt)[#account-number],
    caps-label("IBAN"),           text(font: mono-font, size: 9pt)[#pay.iban],
    caps-label("SWIFT"),          text(font: mono-font, size: 9pt)[#pay.bic_swift],

    table.cell(colspan: 2, inset: (top: 10pt, bottom: 6pt))[
      #align(center)[#bilabel("Correspondent", "Кореспондент")]
    ],

    caps-label("Bank"),           text(size: 9.5pt)[#correspondent-bank],
    caps-label("Account #"),      text(font: mono-font, size: 9pt)[#correspondent-account],
    caps-label("SWIFT"),          text(font: mono-font, size: 9pt)[#correspondent-swift],
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
          #text(size: 8.5pt, fill: ink-muted)[#item.at("description_ua", default: item.description)]
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
      #text(size: 9.5pt, fill: ink-soft)[#amount-in-words] \
      #text(size: 8.5pt, fill: ink-muted)[#amount-in-words-ua]
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
      #text(size: 9pt, fill: ink-soft)[#sender-name-ua]
      #v(0.2cm)
      #line(length: 100%, stroke: 0.5pt + ink)
    ],
  )
]
