# invoice-generator

A CLI tool that generates professional PDF invoices through an interactive prompt session. Built for freelance developers who send monthly invoices and need a fast, repeatable workflow with minimal manual input.

## Table of Contents

- [Features](#features)
- [Prerequisites](#prerequisites)
- [Installation](#installation)
- [Configuration](#configuration)
- [Usage](#usage)
- [Config File Format](#config-file-format)
- [PDF Output](#pdf-output)
- [License](#license)

## Features

- **First-run setup wizard** — interactive walkthrough creates `invoice_config.yaml` from scratch; resumes from where you left off if interrupted
- **YAML-based configuration** — sender, recipients, payment methods, line-item presets, and invoice defaults
- **Config validation** — clear reporting of missing or malformed sections with guidance on how to fix
- **Multiple recipients** — define several client profiles and select by key; set a default for quick invoicing
- **Reusable presets** — define common billable items (description + default daily rate + optional currency and tax rate) and select them by number during invoicing
- **Inline preset creation** — add new presets on the fly during invoice generation without editing the config file
- **Per-preset currency and tax** — each preset can carry its own currency and tax rate, overriding the global default
- **Smart defaults** — billing month defaults to last month, currency to EUR, payment terms to 30 days
- **5 built-in PDF templates** — choose from callisto (bold), leda (clean), thebe (compact), amalthea (high-contrast), or metis (bare-bones)
- **Locale-aware formatting** — dates and numbers in the PDF follow locale rules (en-US, en-GB, de-DE, fr-FR, cs-CZ, uk-UA)
- **Non-interactive CLI mode** — `invoice generate` for scripting and CI; supports single-item (`--preset`/`--days`) or multi-item (`--items` JSON)
- **Preset and recipient management** — `invoice preset list|delete` and `invoice recipient list|add|delete` subcommands
- **Professional PDF output** — clean A4 layout rendered via Typst with line-item table, payment details, and formatted totals
- **Overwrite protection** — prompts before overwriting an existing PDF; standardized filenames (`Invoice_Name_MonYYYY.pdf`)

## Prerequisites

- [Rust](https://www.rust-lang.org/tools/install) 1.85+ (edition 2024)

## Installation

```sh
git clone https://github.com/anton-kochev/invoice-generator.git
cd invoice-generator
cargo build --release
```

The binary will be at `target/release/invoice-generator`.

## Configuration

By default, the config file lives at `~/.config/invoice-generator/config.yaml` (XDG Base Directory specification). The directory is created on first run.

To override the location, use one of (in priority order):

- `--config <PATH>` — CLI flag (works on all subcommands)
- `INVOICE_GENERATOR_CONFIG=<PATH>` — environment variable

### Upgrading from v0.1.0

Earlier versions stored the config as `./invoice_config.yaml` in the current directory. To migrate:

```sh
mkdir -p ~/.config/invoice-generator
mv ./invoice_config.yaml ~/.config/invoice-generator/config.yaml
```

## Usage

```sh
# Interactive mode — setup wizard on first run, then invoice generation
invoice-generator

# Non-interactive: generate a single-item invoice
invoice-generator generate --month 3 --year 2026 --preset dev --days 10

# Non-interactive: multiple line items via JSON
invoice-generator generate --month 3 --year 2026 --items '[{"preset":"dev","days":10},{"preset":"consulting","days":2}]'

# Override template and locale for a single invoice
invoice-generator generate --month 3 --year 2026 --preset dev --days 10 --template amalthea --locale de-DE

# Target a specific client
invoice-generator generate --month 3 --year 2026 --preset dev --days 10 --client acme

# Manage presets
invoice-generator preset list
invoice-generator preset delete old-key

# Manage recipients
invoice-generator recipient list
invoice-generator recipient add
invoice-generator recipient delete old-key
```

On first run, the setup wizard walks you through entering your details, client info, payment methods, and presets. On subsequent runs, you go straight to invoice generation.

### Interactive Flow

```
INVOICE GENERATOR

Month [3]: 3
Year [2026]: 2026

Select a preset for this line item:

  [1] dev — Software development (EUR 800.00/day)
  [2] consulting — Technical consulting (EUR 1000.00/day)
  [3] + Create new preset
Select preset number: 1

Line item #1: Software development
Days worked: 10
Rate per day [800]: 800
  => 10.00 days x 800.00/day = 8000.00

Add another line item? No
```

Before generating the PDF, you see a summary for review:

```
+--------------------------------------+
|          INVOICE SUMMARY             |
+--------------------------------------+
| Invoice:  INV-2026-03                |
| Date:     2026-04-09                 |
| Due:      2026-05-09                 |
+--------------------------------------+
| Software development                 |
|   10.00 days x 800.00 = 8000.00 EUR  |
+--------------------------------------+
| TOTAL: 8000.00 EUR                   |
+--------------------------------------+

Generate PDF? Yes
PDF saved: /path/to/Invoice_Jane_Doe_Mar2026.pdf
```

## Config File Format

The tool stores all static data in a YAML config file (default: `~/.config/invoice-generator/config.yaml` — see [Configuration](#configuration) for overrides). You can edit it by hand or let the setup wizard generate it.

```yaml
sender:
  name: "Jane Doe"
  address:
    - "123 Main Street"
    - "Springfield, IL 62704"
  email: "jane@example.com"

recipients:
  - key: "acme"
    name: "Acme Corp"
    address:
      - "456 Oak Avenue"
      - "Shelbyville, IL 62565"
    company_id: "AC-12345"
    vat_number: "CZ12345678"
  - key: "globex"
    name: "Globex Inc"
    address:
      - "789 Elm Street"
      - "Capital City, IL 62705"

default_recipient: "acme"

payment:
  - label: "SEPA Transfer"
    iban: "DE89370400440532013000"
    bic_swift: "COBADEFFXXX"

presets:
  - key: "dev"
    description: "Software Development"
    default_rate: 100.0
  - key: "consulting"
    description: "Technical Consulting"
    default_rate: 150.0
    currency: "USD"
    tax_rate: 21.0

defaults:
  currency: "EUR"
  payment_terms_days: 30
  invoice_date_day: 9
  template: "leda"
  locale: "en-US"

branding:
  accent_color: "#2563eb"
  footer_text: "Thank you for your business!"
```

### Defaults

| Field | Default | Description |
|-------|---------|-------------|
| `currency` | `EUR` | Currency code used in invoice |
| `payment_terms_days` | `30` | Days until payment is due |
| `invoice_date_day` | `9` | Day of the month for the invoice date (following month) |
| `template` | `leda` | PDF template key (callisto, leda, thebe, amalthea, metis) |
| `locale` | `en-US` | Locale for date/number formatting in PDF (en-US, en-GB, de-DE, fr-FR, cs-CZ, uk-UA) |

All sections except `defaults` and `branding` are required. The `defaults` section is optional and falls back to the values above. Field aliases are supported for convenience (`bic` for `bic_swift`, `vat` for `vat_number`).

Older configs with a single `recipient` field (instead of `recipients` list) are still supported and automatically handled.

## PDF Output

The generated PDF is a single-page A4 document with:

- **Header** — "INVOICE" title with invoice number, date, and due date
- **Parties** — FROM (sender) and TO (recipient) side by side, including optional company ID and VAT number
- **Line items table** — description, period, days, rate, and amount per item with alternating row backgrounds
- **Total** — bold, right-aligned in the configured currency
- **Payment details** — one block per payment method with IBAN and BIC/SWIFT
- **Footer** — thank-you message with sender contact info

### Templates

Five built-in templates control the visual style of the PDF:

| Template | Style |
|----------|-------|
| `callisto` | Bold & structured |
| `leda` | Clean & minimal (default) |
| `thebe` | Compact & dense |
| `amalthea` | High-contrast & vivid |
| `metis` | Bare-bones & printable |

Set the default in config (`defaults.template`) or override per-invoice with `--template` in CLI mode. In interactive mode, you're prompted to change the template before generating.

### Locale Formatting

Dates and numbers in the PDF respect the configured locale. The console UI always remains in English.

| Locale | Date example | Number example |
|--------|-------------|----------------|
| `en-US` | March 9, 2026 | 4,442.40 |
| `en-GB` | 9 March 2026 | 4,442.40 |
| `de-DE` | 9. März 2026 | 4.442,40 |
| `fr-FR` | 9 mars 2026 | 4 442,40 |
| `cs-CZ` | 9. března 2026 | 4 442,40 |
| `uk-UA` | 9 березня 2026 | 4 442,40 |

Filenames follow the pattern `Invoice_{Name}_{MonthAbbrev}{Year}.pdf` (e.g., `Invoice_Jane_Doe_Mar2026.pdf`).

## License

TBD
