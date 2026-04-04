# Software Requirements Specification — Invoice Generator CLI (v2.0)

**Version:** 2.0  
**Date:** 4 April 2026  
**Author:** Claude (Anthropic), based on requirements from Anton Kochiev  
**Parent document:** `SRS_Invoice_Generator.md` (v1.0)

---

## 1. Overview

This document specifies features planned for v2.0 of the Invoice Generator CLI. All v1.0 behavior (described in the parent SRS) remains unchanged unless explicitly noted. v2.0 builds on top of v1.0 — it does not replace it.

---

## 2. Subcommand CLI Architecture

The application adopts a subcommand structure. Running the tool with no subcommand defaults to the interactive invoice flow (v1.0 behavior, §5.2–5.5 of the parent SRS).

### 2.1 Command Reference

| Command | Behavior |
|---------|----------|
| `invoice` | Launch interactive flow (equivalent to v1.0 behavior). |
| `invoice generate --month <M> --year <Y> --preset <key> --days <D>` | Non-interactive invoice generation for a single line item. All flags are required. Outputs the PDF and exits. |
| `invoice generate --month <M> --year <Y> --items '<JSON>'` | Non-interactive with multiple line items (see §2.2). |
| `invoice preset list` | Print all presets from the config file in a formatted table. |
| `invoice preset delete <key>` | Remove a preset by key (see §2.3). |

### 2.2 Multi-Item Non-Interactive Mode

The `--items` flag accepts a JSON array describing one or more line items:

```bash
invoice generate --month 3 --year 2026 \
  --items '[{"preset":"pwc","days":10},{"preset":"internal","days":5,"rate":320}]'
```

- Each object must include `preset` (string, matching a key in config) and `days` (number, > 0).
- The `rate` field is optional; if omitted, the preset's `default_rate` is used.
- `--items` and `--preset`/`--days` are mutually exclusive. Providing both is an error.
- All flags are required — missing flags produce an error message listing what is missing, then exit with a non-zero code.

### 2.3 Preset Deletion

```
$ invoice preset delete pwc
Delete preset "pwc" (Software Development Services PwC Project)? (y/N): y
✓ Preset "pwc" deleted from invoice_config.yaml
```

- Confirms before deleting.
- Refuses if it is the only remaining preset: `Cannot delete — at least one preset must exist.`

---

## 3. Multiple Client Profiles

The `recipient` section of the config becomes a named list of profiles. A `default_recipient` key designates which profile is used when none is specified.

### 3.1 Config Structure

```yaml
recipients:
  - key: "consasoft"
    name: "Consasoft, s.r.o."
    address:
      - "Bělehradská 858/23"
      - "120 00, Prague"
      - "Czech Republic"
    company_id: "29152003"
    vat: "CZW29152003"
  - key: "other_client"
    name: "Other Corp"
    address:
      - "123 Main Street"
      - "London, UK"

default_recipient: "consasoft"
```

### 3.2 Behavior

- **Interactive flow:** If more than one recipient profile exists, prompt the user to select one. If only one exists, use it automatically.
- **CLI mode:** `invoice generate --client consasoft --month 3 ...`. The `--client` flag is optional; if omitted, `default_recipient` is used.
- **Backwards compatibility:** If the config still uses the v1.0 single-`recipient` structure, the application treats it as a single-profile list. No migration is required.

---

## 4. Multi-Currency Support

- The `defaults.currency` field accepts any ISO 4217 code (EUR, USD, GBP, CZK, UAH, etc.).
- Each preset may optionally specify its own `currency` field, overriding the default.
- The PDF renders the correct currency symbol or code in the rate and amount columns.
- No exchange-rate conversion is performed.
- All line items on a single invoice must share the same currency. The application validates this at generation time and rejects mixed-currency invoices with a clear error message.

---

## 5. Tax / VAT Auto-Calculation

### 5.1 Config

Each preset may include an optional `tax_rate` field (percentage):

```yaml
presets:
  - key: "pwc"
    description: "Software Development Services PwC Project"
    default_rate: 360.00
    tax_rate: 21.0    # optional, percentage
```

### 5.2 Behavior

- Tax rate is overridable at runtime during the interactive flow, same as the daily rate.
- In non-interactive mode, each item in the `--items` JSON array may include an optional `tax_rate` field.
- If `tax_rate` is absent or `0`, the row displays no tax and the invoice behaves as in v1.0.

### 5.3 PDF Layout Changes

When at least one line item has a non-zero tax rate:

- The line items table gains a **Tax (%)** and **Tax Amount** column.
- The single **TOTAL** row is replaced by a three-row breakdown: **Subtotal** (sum of net amounts), **Tax** (sum of tax amounts), and **Total** (subtotal + tax).
- Rows with zero tax show "–" in the tax columns.

---

## 6. Custom PDF Branding

An optional `branding` section in the config file controls the visual appearance of the generated PDF.

### 6.1 Config

```yaml
branding:
  logo: "logo.png"           # path to logo image, placed in the header
  accent_color: "#3B82F6"    # hex color for headings and lines
  font: "Helvetica"          # font family override
  footer_text: "Thank you for the opportunity to work together."
```

### 6.2 Behavior

- All fields are optional. Sensible defaults (matching v1.0 styling) apply if the section is absent or partially filled.
- `logo`: image file path, relative to the config file location. Scaled to fit the header area without distorting aspect ratio. Supported formats: PNG, JPEG.
- `accent_color`: applied to the "INVOICE" heading, table header row, and horizontal rule lines.
- `font`: must be a font available to the PDF generation library. Falls back to Helvetica if the specified font is unavailable.
- `footer_text`: replaces the default footer message. Set to an empty string to omit the footer entirely.

---

## 7. Error Handling (v2.0 additions)

| Scenario | Behavior |
|----------|----------|
| Unknown subcommand | Print usage help and exit. |
| Missing required flags in `generate` | List missing flags, print usage for `generate`, exit with non-zero code. |
| `--items` JSON is malformed | Print parse error, exit with non-zero code. |
| `--items` references unknown preset key | Print error naming the unknown key, exit with non-zero code. |
| Mixed currencies in one invoice | Print error listing the conflicting currencies, exit with non-zero code. |
| Logo file not found | Print warning, generate PDF without logo (do not fail). |
| Unknown `--client` key | Print error listing available client profiles, exit with non-zero code. |
| Deleting last remaining preset | Refuse: `Cannot delete — at least one preset must exist.` |

---

## 8. Migration & Backwards Compatibility

- v2.0 must read and work with unmodified v1.0 config files. No manual migration step should be required.
- New config fields (`tax_rate`, `branding`, `recipients`) are all optional and additive.
- Running `invoice` (no subcommand) on a v1.0 config must produce the same result as v1.0.
