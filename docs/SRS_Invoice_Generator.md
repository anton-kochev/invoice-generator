# Software Requirements Specification — Invoice Generator CLI

**Version:** 1.0  
**Date:** 1 April 2026  
**Author:** Claude (Anthropic), based on requirements from Anton Kochiev

---

## 1. Purpose

A console application that generates professional PDF invoices through an interactive prompt session. Designed for a freelance developer who sends monthly invoices to a single client and needs a fast, repeatable workflow with minimal manual input.

---

## 2. Scope

The tool accepts a small set of per-invoice inputs (month, year, line items), combines them with fixed data from a configuration file, and outputs a print-ready PDF. It is a standalone script with no web server, no database, and no GUI.

---

## 3. Definitions

| Term | Meaning |
|------|---------|
| **Line item** | A single billable entry consisting of a description, period, number of days, and daily rate. |
| **Preset** | A reusable line-item template stored in the config file, containing a description and a default daily rate. |
| **Config file** | A YAML file (`invoice_config.yaml`) holding all static invoice data: sender, recipient, payment details, presets, and defaults. |

---

## 4. Configuration File

All static and semi-static data lives in `invoice_config.yaml`. The application must not require code changes for any of the following modifications.

### 4.1 Structure

```yaml
sender:
  name: "Anton Kochiev"
  address:
    - "49000, Ukraine"
    - "Dnipropetrovska region, Dnipro"
    - "st. Naberezhna Peremohy, bld. 130"
    - "housing 7, fl. 19"
  email: "anton.kochev@gmail.com"

recipient:
  name: "Consasoft, s.r.o."
  address:
    - "Bělehradská 858/23"
    - "120 00, Prague"
    - "Czech Republic"
  company_id: "29152003"
  vat: "CZW29152003"

payment:
  sepa:
    label: "SEPA Transfer"
    iban: "GB66CLJU00997187349272"
    bic: "CLJUGB21"
  swift:
    label: "SWIFT Transfer"
    iban: "UA853220010000026008330054176"
    bic: "UNJSUAUKXXX"

presets:
  - key: "pwc"
    description: "Software Development Services PwC Project"
    default_rate: 360.00
  # Add more presets as needed:
  # - key: "internal"
  #   description: "Internal Tooling Development"
  #   default_rate: 320.00

defaults:
  currency: "EUR"
  payment_terms_days: 30
  invoice_date_day: 9        # invoice dated on the 9th of the following month
```

### 4.2 Editability Requirements

- Sender details, recipient details, and payment methods can all be changed by editing the config file — no code changes required.
- Presets can be added, removed, or reordered freely — both by editing the file and through the interactive CLI (see §5.1, §5.3).
- Default rate per preset is overridable at runtime (see §5.4).
- If `invoice_config.yaml` does not exist on launch, the application enters a first-run setup flow (§5.1) that collects all required data interactively and writes the config file. The user never needs to create the file by hand.

---

## 5. Interactive Console Flow

When launched, the application first checks for a config file. If absent, it enters the first-run setup (§5.1). If present, it skips directly to the invoice flow (§5.2).

### 5.1 First-Run Setup

Triggered when `invoice_config.yaml` does not exist in the expected location. The CLI walks through each config section, collecting all required fields interactively. After each section, the data is written to disk so that progress is not lost if the user aborts mid-setup.

#### 5.1.1 Sender Details

```
═══════════════════════════════════════
  INVOICE GENERATOR — First-time setup
═══════════════════════════════════════

Let's set up your invoice profile.

Your details (sender)
  Full name: █
  Address line 1: █
  Address line 2 (blank to finish): █
  ...
  Email: █
```

- At least one address line is required.
- The user keeps entering address lines until they submit a blank line.

#### 5.1.2 Recipient Details

```
Client details (recipient)
  Company name: █
  Address line 1: █
  Address line 2 (blank to finish): █
  ...
  Company ID (blank to skip): █
  VAT number (blank to skip): █
```

- Company ID and VAT are optional (some clients may not have them).

#### 5.1.3 Payment Details

```
Payment methods
  How many payment methods? [2]: █

Payment method #1
  Label (e.g. "SEPA Transfer"): █
  IBAN: █
  BIC/SWIFT: █

Payment method #2
  Label (e.g. "SWIFT Transfer"): █
  IBAN: █
  BIC/SWIFT: █
```

- At least one payment method is required.
- The user specifies how many methods, then fills in each.

#### 5.1.4 First Preset

```
Let's create your first line-item preset.

  Short key (e.g. "pwc"): █
  Description (e.g. "Software Development Services PwC Project"): █
  Default daily rate (EUR): █
```

- At least one preset is required to generate an invoice.
- After the first preset: `Add another preset? (y/N):`
- If yes, the same three prompts repeat for each additional preset.

#### 5.1.5 Defaults

```
Invoice defaults
  Currency [EUR]: █
  Invoice date — day of the following month [9]: █
  Payment terms in days [30]: █
```

- All three fields have sensible defaults shown in brackets; the user can press Enter to accept.

#### 5.1.6 Setup Complete

```
✓ Config saved to invoice_config.yaml

  Sender:   Anton Kochiev
  Client:   Consasoft, s.r.o.
  Presets:  1 — pwc (€360.00/day)
  Terms:    NET 30, invoiced on the 9th

You can edit these anytime in invoice_config.yaml.
Proceeding to invoice generation...
```

The application then continues to §5.2.

### 5.2 Invoice Period

```
═══════════════════════════════════════
  INVOICE GENERATOR
═══════════════════════════════════════

Invoice period
  Month [2] (1–12): █
  Year [2026]: █
```

- Defaults: current month − 1 for month, current year for year (adjusted if January).
- Validates month (1–12) and year (reasonable range).

### 5.3 Line Items — Preset Selection

```
Available presets:
  [1] pwc — Software Development Services PwC Project (€360.00/day)
  [2] internal — Internal Tooling Development (€320.00/day)
  [N] + Create new preset

Select preset for line item #1: █
```

- User picks an existing preset by number, or selects `N` to create a new one.
- After each line item is complete (see §5.4), prompt: `Add another line item? (y/N):`

#### Creating a New Preset Inline

If the user selects `N` (new preset), the same preset creation flow from §5.1.4 runs:

```
New preset
  Short key (e.g. "consulting"): █
  Description: █
  Default daily rate (EUR): █

✓ Preset "consulting" saved to invoice_config.yaml
```

- The new preset is immediately appended to the config file and added to the in-memory preset list.
- After creation, it is automatically selected for the current line item, and the flow continues to §5.4 (line item details).
- The preset is available for selection in all future runs without re-entering it.

### 5.4 Line Item Details

```
Line item #1: Software Development Services PwC Project
  Days worked: █
  Rate (EUR/day) [360.00]: █
```

- **Days worked:** required, accepts decimals (e.g. `12.34`). Must be > 0.
- **Rate:** defaults to the preset's `default_rate`. User can press Enter to accept or type a different value.
- Amount is computed as `days × rate`, rounded to 2 decimal places (half-up).

### 5.5 Confirmation

```
┌─────────────────────────────────────────────────┐
│ Invoice INV-2026-02                             │
│ Date: 9 March 2026    Due: 8 April 2026         │
│                                                 │
│ 1. Software Dev Services PwC   12.34d × €360.00 │
│                                    = €4,442.40  │
│                                                 │
│ TOTAL: €4,442.40                                │
└─────────────────────────────────────────────────┘

Generate PDF? (Y/n): █
```

- Shows a summary of all line items and the total.
- On confirmation, generates the PDF and prints the output file path.
- On rejection, returns to §5.2.

---

## 6. Invoice Number & Dates

| Field | Rule |
|-------|------|
| **Invoice number** | `INV-{YYYY}-{MM}` where YYYY and MM are the billed period's year and month. |
| **Invoice date** | The `invoice_date_day` of the month following the billed period (configurable in config, default: 9th). |
| **Due date** | Invoice date + `payment_terms_days` (configurable, default: 30 days). |

---

## 7. PDF Output

### 7.1 File Naming

`Invoice_Anton_Kochiev_{MonthName}{YYYY}.pdf`

Example: `Invoice_Anton_Kochiev_Feb2026.pdf`

Output directory: current working directory.

### 7.2 Layout

Single-page A4 portrait. The layout must be clean and professional but does not need to replicate the original invoice pixel-for-pixel.

Required sections, top to bottom:

1. **Header** — "INVOICE" title, invoice number, invoice date, due date.
2. **Parties** — "FROM" block (sender) and "TO" block (recipient with company ID and VAT), side by side.
3. **Line items table** — columns: Description, Period, Days, Rate (EUR/MD), Amount (EUR). One row per line item. Period is always `"{MonthName} {YYYY}"` matching the billed period.
4. **Total row** — bold, right-aligned, showing `TOTAL EUR {amount}`.
5. **Payment details** — SEPA and SWIFT blocks showing IBAN and BIC/SWIFT for each.
6. **Footer** — "Thank you for the opportunity to work together." followed by sender name and email.

### 7.3 Styling

- Clean sans-serif font (Helvetica or similar built-in).
- Subtle use of color for accents (headers, lines) is acceptable.
- Table should have light row separators or alternating backgrounds for readability.
- No heavy borders or ornamental elements.

---

## 8. Technical Constraints

- **Language:** Rust — compiles to a single static binary with no runtime dependencies.
- **Key crates:** `serde` + `serde_yaml` (config), `dialoguer` (interactive prompts), `genpdf` or `printpdf` (PDF), `chrono` (dates).
- The application is a standalone console program — no web server, no database, no GUI.
- No external services or network access required at runtime.
- Must be distributable as a single binary. The config file is generated on first run and does not need to be shipped alongside the binary.
- PDF generation library must support A4 page size, embedded fonts, and basic vector drawing (lines, rectangles, colored text).
- Config file format: YAML.
- Year input validated to range 2000–2099.

---

## 9. Error Handling

| Scenario | Behavior |
|----------|----------|
| Config file missing | Enter first-run setup flow (§5.1) to create it interactively. |
| Config file malformed | Print YAML parse error with line number and exit. |
| Config file missing required fields | Print which section is incomplete and exit with guidance to fix or delete the file to re-run setup. |
| Duplicate preset key during creation | Reject and re-prompt: `Key "pwc" already exists. Choose another:` |
| Invalid input (non-numeric days, out-of-range month) | Re-prompt with explanation; do not crash. |
| Output file already exists | Ask to overwrite: `File already exists. Overwrite? (y/N):` |
| Zero line items | Do not allow invoice generation; re-prompt to add at least one. |

---

## 10. Out of Scope

The following are explicitly **not** included in v1.0:

- Invoice history tracking or database.
- Multiple clients (recipient is fixed in config).
- Email sending or any network features.
- GUI or web interface.
- Tax calculations (VAT, withholding, etc.).
- Multi-currency support (EUR only).
- Digital signatures or e-invoicing standards.
- Localization / i18n.

---

## 11. Future Considerations

These may be added in later versions if needed:

- CLI argument mode for scripting (`--month 3 --year 2026 --preset pwc --days 12.34`).
- Invoice history log (JSON/CSV append) for record-keeping.
- Multiple recipient profiles in config.
- Custom PDF templates / branding options.
