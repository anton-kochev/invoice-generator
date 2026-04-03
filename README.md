# invoice-generator

A CLI tool that generates professional PDF invoices through an interactive prompt session. Built for freelance developers who send monthly invoices and need a fast, repeatable workflow with minimal manual input.

## Table of Contents

- [Features](#features)
- [Prerequisites](#prerequisites)
- [Installation](#installation)
- [Usage](#usage)
- [Configuration](#configuration)
- [Project Status](#project-status)
- [License](#license)

## Features

- YAML-based configuration for sender, recipient, payment details, and line-item presets
- Config validation with clear reporting of missing sections
- Reusable presets for common billable items (description + default daily rate)
- Field aliases for convenience (`bic` for `bic_swift`, `vat` for `vat_number`)
- Serde defaults for sensible out-of-the-box values (EUR, 30-day payment terms)

### Planned

- Interactive setup wizard for first-run configuration
- Prompt-driven invoice generation (month, year, line items)
- PDF output with print-ready formatting
- Preset management (add/edit/delete via CLI)

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
# Run from a directory containing invoice_config.yaml
invoice-generator
```

The tool looks for `invoice_config.yaml` in the current working directory. If the file is missing, it will guide you through setup (planned). If present, it validates the config and reports any missing sections.

## Configuration

Create an `invoice_config.yaml` in your working directory:

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

| Field | Default |
|-------|---------|
| `currency` | `EUR` |
| `payment_terms_days` | `30` |
| `invoice_date_day` | `9` |

All sections except `defaults` are required for invoice generation. The `defaults` section is optional and falls back to the values above.

## Project Status

Early development. Config loading, validation, and persistence are implemented and tested. See [docs/user-stories-invoice-generator.md](docs/user-stories-invoice-generator.md) for the full roadmap.

## License

TBD
