# invoice-generator

A CLI tool that generates professional PDF invoices through an interactive prompt session. Built for freelance developers who send monthly invoices and need a fast, repeatable workflow with minimal manual input.

## Table of Contents

- [Features](#features)
- [Prerequisites](#prerequisites)
- [Installation](#installation)
- [Usage](#usage)
- [Configuration](#configuration)
- [PDF Output](#pdf-output)
- [License](#license)

## Features

- **First-run setup wizard** — interactive walkthrough creates `invoice_config.yaml` from scratch; resumes from where you left off if interrupted
- **YAML-based configuration** — sender, recipient, payment methods, line-item presets, and invoice defaults
- **Config validation** — clear reporting of missing or malformed sections with guidance on how to fix
- **Reusable presets** — define common billable items (description + default daily rate) and select them by number during invoicing
- **Inline preset creation** — add new presets on the fly during invoice generation without editing the config file
- **Smart defaults** — billing month defaults to last month, currency to EUR, payment terms to 30 days
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

## Usage

```sh
# Run from the directory where you want invoice_config.yaml and PDFs
invoice-generator
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

## Configuration

The tool stores all static data in `invoice_config.yaml` in the current working directory. You can edit it by hand or let the setup wizard generate it.

```yaml
sender:
  name: "Jane Doe"
  address:
    - "123 Main Street"
    - "Springfield, IL 62704"
  email: "jane@example.com"

recipient:
  name: "Acme Corp"
  address:
    - "456 Oak Avenue"
    - "Shelbyville, IL 62565"
  company_id: "AC-12345"
  vat_number: "CZ12345678"

payment:
  - label: "SEPA Transfer"
    iban: "DE89370400440532013000"
    bic_swift: "COBADEFFXXX"

presets:
  - key: "dev"
    description: "Software Development"
    default_rate: 100.0

defaults:
  currency: "EUR"
  payment_terms_days: 30
  invoice_date_day: 9
```

### Defaults

| Field | Default | Description |
|-------|---------|-------------|
| `currency` | `EUR` | Currency code used in invoice |
| `payment_terms_days` | `30` | Days until payment is due |
| `invoice_date_day` | `9` | Day of the month for the invoice date (following month) |

All sections except `defaults` are required. The `defaults` section is optional and falls back to the values above. Field aliases are supported for convenience (`bic` for `bic_swift`, `vat` for `vat_number`).

## PDF Output

The generated PDF is a single-page A4 document with:

- **Header** — "INVOICE" title with invoice number, date, and due date
- **Parties** — FROM (sender) and TO (recipient) side by side, including optional company ID and VAT number
- **Line items table** — description, period, days, rate, and amount per item with alternating row backgrounds
- **Total** — bold, right-aligned in the configured currency
- **Payment details** — one block per payment method with IBAN and BIC/SWIFT
- **Footer** — thank-you message with sender contact info

Filenames follow the pattern `Invoice_{Name}_{MonthAbbrev}{Year}.pdf` (e.g., `Invoice_Jane_Doe_Mar2026.pdf`).

## License

TBD
