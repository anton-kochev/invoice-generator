# User Stories — Invoice Generator CLI

## Summary
- **Epics**: 11 (5 v1.0 + 6 v2.0)
- **Total User Stories**: 41 (24 completed ✅ + 17 remaining)
- **User Roles Identified**: Freelance Developer (sole actor — referred to as "user" throughout)

---

## Epic 1: Config File Management
> Foundation for reading, validating, and persisting the YAML configuration that drives all invoice data.

### Story 1.1: Load and Parse Config File ✅
**As a** user,
**I want** the app to load `invoice_config.yaml` from the working directory at startup,
**So that** all my static invoice data is ready without re-entering it each time.

**Acceptance Criteria:**
- [ ] Reads `invoice_config.yaml` from the current working directory on launch
- [ ] Parses YAML into an in-memory structure covering all sections: `sender`, `recipient`, `payment`, `presets`, `defaults`
- [ ] If the file does not exist, control passes to the first-run setup flow (Story 2.1) — no crash
- [ ] If the file exists and is valid, control passes to the invoice flow (Story 3.1)

**Dependencies:** None

---

### Story 1.2: Validate Config Completeness ✅
**As a** user,
**I want** the app to tell me exactly which config section is broken or missing,
**So that** I can fix it without guessing.

**Acceptance Criteria:**
- [ ] If YAML parsing fails, prints the parse error with line number and exits with non-zero code
- [ ] If required fields are missing (sender.name, recipient.name, at least one payment method, at least one preset), prints which section is incomplete
- [ ] If config is partially complete (e.g., user aborted mid-setup), resumes setup from the first missing section rather than rejecting the file
- [ ] Error message includes guidance: "Fix the file or delete it to re-run setup"
- [ ] Does not crash on unexpected extra fields — ignores them gracefully

**Dependencies:** Story 1.1

---

### Story 1.3: Persist Config Changes to YAML ✅
**As a** user,
**I want** config changes (e.g., new presets) to be written back to `invoice_config.yaml`,
**So that** they survive across sessions without manual file editing.

**Acceptance Criteria:**
- [ ] Can write the full config structure to `invoice_config.yaml`
- [ ] Can append a new preset to the existing file without clobbering other sections
- [ ] Written file is valid YAML and re-parseable by Story 1.1
- [ ] During first-run setup, each section is written to disk immediately after collection so partial progress is not lost if the user aborts (§5.1)

**Dependencies:** Story 1.1

---

## Epic 2: First-Run Setup Wizard
> Interactive walkthrough that creates the config file from scratch when none exists.

### Story 2.1: Detect Missing Config and Launch Setup ✅
**As a** user running the app for the first time,
**I want** an interactive setup wizard to start automatically,
**So that** I never have to create `invoice_config.yaml` by hand.

**Acceptance Criteria:**
- [ ] When `invoice_config.yaml` does not exist, displays the header: `INVOICE GENERATOR — First-time setup`
- [ ] When `invoice_config.yaml` exists but is incomplete (partial setup from a previous abort), resumes setup from the first missing section
- [ ] Proceeds to sender details collection (Story 2.2) — or the first incomplete section if resuming
- [ ] Does not prompt for setup if a valid config already exists

**Dependencies:** Story 1.1

---

### Story 2.2: Collect Sender Details ✅
**As a** user,
**I want** to enter my name, address, and email during setup,
**So that** my details appear correctly on every invoice.

**Acceptance Criteria:**
- [ ] Prompts for: full name (required), address lines (at least one required, blank line terminates), email (required)
- [ ] Saves the `sender` section to disk immediately after completion
- [ ] Re-prompts if name or email is left blank

**Dependencies:** Story 2.1, Story 1.3

---

### Story 2.3: Collect Recipient Details ✅
**As a** user,
**I want** to enter my client's company info during setup,
**So that** the invoice is correctly addressed.

**Acceptance Criteria:**
- [ ] Prompts for: company name (required), address lines (at least one required, blank terminates), company ID (optional — blank to skip), VAT number (optional — blank to skip)
- [ ] Saves the `recipient` section to disk immediately after completion

**Dependencies:** Story 2.2, Story 1.3

---

### Story 2.4: Collect Payment Methods ✅
**As a** user,
**I want** to enter one or more payment methods during setup,
**So that** clients know how to pay me.

**Acceptance Criteria:**
- [ ] Asks "How many payment methods?" with default of 2
- [ ] For each method, prompts: label (e.g. "SEPA Transfer"), IBAN, BIC/SWIFT — all required
- [ ] At least one payment method must be entered; re-prompts if user enters 0
- [ ] Saves the `payment` section to disk immediately after completion

**Dependencies:** Story 2.3, Story 1.3

---

### Story 2.5: Create Initial Presets ✅
**As a** user,
**I want** to define at least one line-item preset during setup,
**So that** I can quickly select it when generating invoices.

**Acceptance Criteria:**
- [ ] Prompts for first preset: short key (e.g. "pwc"), description, default daily rate in the configured currency
- [ ] After first preset, asks "Add another preset? (y/N)"
- [ ] Repeats for each additional preset until user declines
- [ ] Validates rate is a positive number; re-prompts on invalid input
- [ ] Saves the `presets` section to disk after completion

**Dependencies:** Story 2.4, Story 1.3

---

### Story 2.6: Set Invoice Defaults ✅
**As a** user,
**I want** to configure currency, invoice date day, and payment terms with sensible defaults,
**So that** I can just press Enter for the common case.

**Acceptance Criteria:**
- [ ] Prompts for: currency (default "EUR"), invoice date day of following month (default 9), payment terms in days (default 30)
- [ ] Pressing Enter accepts the bracketed default
- [ ] Saves the `defaults` section to disk after completion

**Dependencies:** Story 2.5, Story 1.3

---

### Story 2.7: Display Setup Summary and Proceed ✅
**As a** user,
**I want** to see a summary of my config after setup completes,
**So that** I can confirm everything looks right before generating invoices.

**Acceptance Criteria:**
- [ ] Displays: sender name, client name, number of presets with keys and rates, payment terms
- [ ] Shows message: "You can edit these anytime in invoice_config.yaml."
- [ ] Shows: "Proceeding to invoice generation..."
- [ ] Transitions to invoice period prompt (Story 3.1)

**Dependencies:** Story 2.6

---

## Epic 3: Invoice Data Collection
> The interactive prompts that gather period, line items, and confirmation for a single invoice.

### Story 3.1: Prompt for Invoice Period ✅
**As a** user,
**I want** to enter the billing month and year with smart defaults,
**So that** generating last month's invoice requires minimal typing.

**Acceptance Criteria:**
- [ ] Displays header: `INVOICE GENERATOR`
- [ ] Prompts for month (1–12) defaulting to current month minus 1
- [ ] Prompts for year defaulting to current year (adjusted to previous year if current month is January and default month becomes 12)
- [ ] Validates year is within 2000–2099; re-prompts with explanation on out-of-range values
- [ ] Re-prompts with explanation on out-of-range month or invalid year
- [ ] Accepts the input and proceeds to line item selection

**Dependencies:** Story 1.1

---

### Story 3.2: Select Preset for Line Item ✅
**As a** user,
**I want** to pick a preset from a numbered list when adding a line item,
**So that** I don't have to re-type descriptions and rates every time.

**Acceptance Criteria:**
- [ ] Lists all presets as `[N] key — description (currency rate/day)`
- [ ] Shows option `[N] + Create new preset` as the last entry
- [ ] User selects by number; invalid numbers re-prompt
- [ ] Selected preset's description and default rate carry forward to line item details (Story 3.4)

**Dependencies:** Story 1.1, Story 3.1

---

### Story 3.3: Create New Preset Inline ✅
**As a** user,
**I want** to create a new preset on the fly during invoice generation,
**So that** I don't have to abort and edit the config file when I have a new type of work.

**Acceptance Criteria:**
- [ ] When user selects "Create new preset", prompts for: short key, description, default daily rate
- [ ] Rejects duplicate keys with message `Key "X" already exists. Choose another:` and re-prompts
- [ ] Appends the new preset to `invoice_config.yaml` immediately
- [ ] Displays confirmation: `Preset "key" saved to invoice_config.yaml`
- [ ] Automatically selects the new preset for the current line item and proceeds to Story 3.4
- [ ] New preset appears in the list for subsequent line items and future runs

**Dependencies:** Story 3.2, Story 1.3

---

### Story 3.4: Enter Line Item Details ✅
**As a** user,
**I want** to enter days worked and optionally override the rate for each line item,
**So that** I can accurately bill for the actual work done.

**Acceptance Criteria:**
- [ ] Displays: `Line item #N: {preset description}`
- [ ] Prompts for days worked (required, decimal allowed e.g. `12.34`, must be > 0)
- [ ] Prompts for rate with preset's default shown in brackets; Enter accepts default
- [ ] Computes amount as `days * rate`, rounded to 2 decimal places using half-up rounding
- [ ] Re-prompts on non-numeric or non-positive input for days or rate

**Dependencies:** Story 3.2

---

### Story 3.5: Support Multiple Line Items ✅
**As a** user,
**I want** to add as many line items as I need on one invoice,
**So that** I can bill for multiple workstreams in a single month.

**Acceptance Criteria:**
- [ ] After each line item, asks: `Add another line item? (y/N)`
- [ ] If yes, returns to preset selection (Story 3.2) with incremented item number
- [ ] If no, proceeds to confirmation (Story 3.6)
- [ ] At least one line item is required — if somehow zero, re-prompts (should not happen in normal flow)

**Dependencies:** Story 3.4

---

### Story 3.6: Display Invoice Summary and Confirm ✅
**As a** user,
**I want** to review a formatted summary of the invoice before generating the PDF,
**So that** I can catch mistakes without wasting time on a bad PDF.

**Acceptance Criteria:**
- [ ] Displays boxed summary showing: invoice number (INV-YYYY-MM), invoice date, due date
- [ ] Lists each line item with days, rate, and computed amount
- [ ] Shows total amount (sum of all line item amounts)
- [ ] Asks: `Generate PDF? (Y/n)`
- [ ] On "Y" or Enter, proceeds to PDF generation (Story 4.1)
- [ ] On "n", returns to invoice period prompt (Story 3.1) to start over

**Dependencies:** Story 3.5

---

## Epic 4: PDF Generation
> Rendering the collected invoice data into a professional, print-ready A4 PDF file.

### Story 4.1: Compute Invoice Number and Dates ✅
**As a** user,
**I want** invoice number, date, and due date calculated automatically from the billing period,
**So that** I don't have to compute dates manually.

**Acceptance Criteria:**
- [ ] Invoice number follows format `INV-{YYYY}-{MM}` using the billed period's year and zero-padded month
- [ ] Invoice date is the `invoice_date_day` (from config) of the month following the billed period
- [ ] Due date is invoice date + `payment_terms_days` (from config)
- [ ] Handles year boundaries correctly (e.g., billing December 2025 → invoice date January 2026)

**Dependencies:** Story 3.6

---

### Story 4.2: Render PDF Layout ✅
**As a** user,
**I want** the PDF to contain all required sections in a clean, professional layout,
**So that** I can send it to my client without further editing.

**Acceptance Criteria:**
- [ ] Single-page A4 portrait
- [ ] **Header**: "INVOICE" title, invoice number, invoice date, due date
- [ ] **Parties**: "FROM" block (sender name, address, email) and "TO" block (recipient name, address, company ID, VAT) side by side
- [ ] **Line items table**: columns — Description, Period (`{MonthName} {YYYY}`), Days, Rate (EUR/MD), Amount (EUR); one row per line item
- [ ] **Total row**: bold, right-aligned, format `TOTAL EUR {amount}`
- [ ] **Payment details**: one block per payment method showing label, IBAN, BIC/SWIFT
- [ ] **Footer**: "Thank you for the opportunity to work together." followed by sender name and email
- [ ] Clean sans-serif font (Helvetica or similar built-in)
- [ ] Subtle color accents on headers/lines; light row separators or alternating backgrounds on the table
- [ ] No heavy borders or ornamental elements
- [ ] Layout must be deterministic — identical input data always produces an identical PDF

**Dependencies:** Story 4.1

---

### Story 4.3: Save PDF with Correct Filename ✅
**As a** user,
**I want** the PDF saved with a standardized filename in the current directory,
**So that** my invoices are consistently named and easy to find.

**Acceptance Criteria:**
- [ ] Filename follows pattern: `Invoice_{Name}_{MonthAbbrev}{YYYY}.pdf` where `{Name}` is the sender's full name with spaces replaced by underscores (e.g., `Invoice_Anton_Kochiev_Feb2026.pdf`)
- [ ] Saved to current working directory
- [ ] If file already exists, asks: `File already exists. Overwrite? (y/N)`
- [ ] On "y", overwrites; on "N" or Enter, aborts generation and returns to the flow
- [ ] Prints the full output file path on success

**Dependencies:** Story 4.2

---

## Epic 5: Error Handling & Input Validation
> Cross-cutting robustness: graceful re-prompts, clear error messages, no crashes on bad input.

### Story 5.1: Validate All Numeric Inputs ✅
**As a** user,
**I want** the app to re-prompt with a clear explanation when I enter invalid data,
**So that** I can correct mistakes without the app crashing.

**Acceptance Criteria:**
- [ ] Non-numeric input for days, rate, month, year, payment count → re-prompt with explanation (e.g., "Please enter a valid number")
- [ ] Days ≤ 0 → re-prompt: "Days must be greater than 0"
- [ ] Month outside 1–12 → re-prompt: "Month must be between 1 and 12"
- [ ] Year outside 2000–2099 → re-prompt: "Year must be between 2000 and 2099"
- [ ] Rate ≤ 0 → re-prompt: "Rate must be greater than 0"
- [ ] App never crashes or exits on invalid interactive input — always re-prompts

**Dependencies:** None (cross-cutting — implemented alongside each input story)

---

## Resolved Gaps & Ambiguities

| # | SRS Section | Issue | Resolution |
|---|-------------|-------|------------|
| 1 | §7.1 — File naming | `output_dir` mentioned as configurable but absent from config schema | **Always use cwd.** Removed `output_dir` from scope — PDFs save to current working directory. |
| 2 | §5.2 — Year validation | "Reasonable range" for year is undefined | **2000–2099.** Added to Stories 3.1 and 5.1 acceptance criteria. |
| 3 | §7.2 — PDF layout | No reference design; layout "does not need to replicate pixel-for-pixel" | **Deterministic layout.** Same input always produces the same PDF. Added to Story 4.2. |
| 4 | §7.1 — Sender name in filename | Config stores single `name` field — unclear how to handle multi-part names | **Replace spaces with underscores** in full name. e.g., `Anton Kochiev` → `Anton_Kochiev`. Updated Story 4.3. |
| 5 | §5.1 — Partial config on abort | Unclear behavior when config is partially written from an aborted setup | **Resume incomplete setup** from first missing section on next launch. Updated Stories 1.2 and 2.1. |
| 6 | §8 — Technology stack | No programming language or PDF library specified | **Rust** — see Technology Stack section below. |

---

## Technology Stack

| Concern | Crate |
|---------|-------|
| YAML config | `serde` + `serde_yaml` (yaml_serde 0.10) |
| Interactive prompts | `inquire` |
| PDF generation | `typst` + `typst-kit` + `typst-pdf` |
| Date handling | `time` |
| CLI argument parsing | `clap` (derive mode) |
| Error handling | `thiserror` |
| JSON parsing | `serde_json` |
| Memoization (Typst) | `comemo` |
| Test utilities | `tempfile` (dev-dependency) |

The application compiles to a single static binary with no runtime dependencies.

---

## Dependency Map

**Critical path** (longest chain — this dictates minimum calendar time):

```
1.1 (Load config) → 1.3 (Persist config) → 2.1 (Detect & launch setup)
  → 2.2 → 2.3 → 2.4 → 2.5 → 2.6 → 2.7 (Setup wizard sequence)
    → 3.1 (Invoice period) → 3.2 (Select preset) → 3.4 (Line item details)
      → 3.5 (Multiple items) → 3.6 (Confirm)
        → 4.1 (Compute dates) → 4.2 (Render PDF) → 4.3 (Save file)
```

**Parallel tracks** (can be developed alongside the critical path):

- `1.2 (Validate config)` — depends only on 1.1, can be built in parallel with Epic 2
- `3.3 (Inline preset creation)` — depends on 3.2 + 1.3, can be deferred after core flow works
- `5.1 (Input validation)` — cross-cutting, incrementally added alongside each input story

**Recommended sprint ordering:**

1. **Sprint 1** ✅: Stories 1.1, 1.2, 1.3 (config foundation) + Rust project scaffolding
2. **Sprint 2** ✅: Stories 2.1–2.7 (full setup wizard)
3. **Sprint 3** ✅: Stories 3.1, 3.2, 3.4, 3.5, 3.6 (core invoice flow) + 5.1 (validation)
4. **Sprint 4** ✅: Stories 4.1, 4.2, 4.3 (PDF generation) + 3.3 (inline preset creation)

---

# v2.0 User Stories

> All stories below build on top of the completed v1.0 foundation. v1.0 behavior remains unchanged unless explicitly noted. See `docs/SRS_Invoice_Generator_v2.md` for the full specification.

---

## Epic 6: Subcommand CLI Architecture
> Introduce a subcommand structure so the tool can be used both interactively and non-interactively from scripts.

### Story 6.1: Subcommand Routing and Default Behavior ✅
**As a** user,
**I want** the CLI to support subcommands (`invoice`, `invoice generate`, `invoice preset`),
**So that** I can choose between interactive and scripted workflows.

**Acceptance Criteria:**
- [ ] Running `invoice` with no subcommand launches the existing interactive flow (v1.0 behavior, unchanged)
- [ ] Unknown subcommands print usage help and exit with non-zero code
- [ ] `--help` on any subcommand prints usage for that specific subcommand
- [ ] Existing v1.0 behavior is fully preserved when no subcommand is given

**Dependencies:** None (builds on existing main.rs entry point)

---

### Story 6.2: Non-Interactive Single-Item Generation ✅
**As a** user,
**I want** to generate an invoice from the command line with `--month`, `--year`, `--preset`, and `--days` flags,
**So that** I can script invoice generation without interactive prompts.

**Acceptance Criteria:**
- [ ] `invoice generate --month 3 --year 2026 --preset pwc --days 10` generates a PDF and exits
- [ ] All four flags are required — missing flags produce an error listing what is missing, then exit with non-zero code
- [ ] `--preset` value must match an existing preset key; unknown key prints error and exits with non-zero code
- [ ] `--days` must be > 0; invalid value prints error and exits
- [ ] Output PDF uses the same filename convention as interactive mode (Story 4.3)
- [ ] If a PDF with the same filename already exists, silently overwrites it
- [ ] Exit code is 0 on success

**Dependencies:** Story 6.1

---

### Story 6.3: Non-Interactive Multi-Item Generation ✅
**As a** user,
**I want** to pass multiple line items as JSON via `--items` flag,
**So that** I can generate multi-line invoices in a single command.

**Acceptance Criteria:**
- [ ] `--items '[{"preset":"pwc","days":10},{"preset":"internal","days":5}]'` generates a PDF with two line items
- [ ] Each JSON object must include `preset` (string) and `days` (number > 0)
- [ ] Optional `rate` field overrides the preset's `default_rate` (number only — currency always comes from preset/default)
- [ ] `--items` and `--preset`/`--days` are mutually exclusive — providing both prints error and exits
- [ ] Malformed JSON prints parse error and exits with non-zero code
- [ ] Unknown preset key in any item prints error naming the unknown key and exits
- [ ] All other required flags (`--month`, `--year`) still apply

**Dependencies:** Story 6.2

---

### Story 6.4: Preset Listing Subcommand ✅
**As a** user,
**I want** to run `invoice preset list` to see all configured presets,
**So that** I can check preset keys and rates without opening the config file.

**Acceptance Criteria:**
- [ ] Prints a formatted table with columns: Key, Description, Default Rate, Currency
- [ ] Lists all presets from the config file
- [ ] Exits with code 0

**Dependencies:** Story 6.1

---

### Story 6.5: Preset Deletion Subcommand ✅
**As a** user,
**I want** to run `invoice preset delete <key>` to remove a preset,
**So that** I can clean up presets I no longer use.

**Acceptance Criteria:**
- [ ] Prompts for confirmation: `Delete preset "pwc" (Software Development Services PwC Project)? (y/N)`
- [ ] On "y", removes the preset from `invoice_config.yaml` and prints `✓ Preset "pwc" deleted from invoice_config.yaml`
- [ ] On "N" or Enter, aborts without changes
- [ ] If the preset is the only remaining one, refuses: `Cannot delete — at least one preset must exist.`
- [ ] Unknown key prints error and exits with non-zero code

**Dependencies:** Story 6.1, Story 1.3

---

## Epic 7: Multiple Client Profiles
> Support multiple recipients in the config so users who invoice different clients don't need multiple config files.

### Story 7.1: Multi-Recipient Config Structure
**As a** user,
**I want** the config file to support a named list of recipients with a default,
**So that** I can store multiple client profiles in one config.

**Acceptance Criteria:**
- [ ] Config supports a `recipients` array, each entry having a `key`, `name`, `address`, and optional `company_id`/`vat`
- [ ] A `default_recipient` field designates the default profile key
- [ ] Backwards compatible: if config uses v1.0 single-`recipient` structure, treats it as a single-profile list — no migration required
- [ ] Config validation ensures at least one recipient exists and `default_recipient` references a valid key

**Dependencies:** Story 1.1, Story 1.2

---

### Story 7.2: Recipient Selection in Interactive Flow
**As a** user,
**I want** to be prompted to select a recipient when multiple profiles exist,
**So that** I can invoice the right client each time.

**Acceptance Criteria:**
- [ ] If only one recipient exists, uses it automatically (no prompt)
- [ ] If multiple recipients exist, displays a numbered list and prompts for selection
- [ ] Selected recipient's data is used for the invoice "TO" section and PDF
- [ ] Works with both v1.0 and v2.0 config formats

**Dependencies:** Story 7.1

---

### Story 7.3: Client Flag in Non-Interactive Mode
**As a** user,
**I want** to specify `--client <key>` in `invoice generate` to choose a recipient,
**So that** scripted generation works with multi-client configs.

**Acceptance Criteria:**
- [ ] `--client` flag is optional; if omitted, `default_recipient` is used
- [ ] Unknown `--client` key prints error listing available client profiles and exits with non-zero code
- [ ] Works with both single-item and multi-item (`--items`) generation

**Dependencies:** Story 7.1, Story 6.2

---

### Story 7.4: Recipient Listing Subcommand
**As a** user,
**I want** to run `invoice recipient list` to see all configured client profiles,
**So that** I can check recipient keys without opening the config file.

**Acceptance Criteria:**
- [ ] Prints a formatted table with columns: Key, Name, Address (first line), Company ID
- [ ] Marks the default recipient with an indicator (e.g., `*` or `(default)`)
- [ ] Lists all recipients from the config file
- [ ] Exits with code 0

**Dependencies:** Story 7.1

---

### Story 7.5: Recipient Add Subcommand
**As a** user,
**I want** to run `invoice recipient add` to interactively add a new client profile,
**So that** I don't have to hand-edit the YAML config.

**Acceptance Criteria:**
- [ ] Prompts for: key (short identifier), company name (required), address lines (at least one, blank terminates), company ID (optional), VAT (optional)
- [ ] Rejects duplicate keys with message `Key "X" already exists. Choose another:` and re-prompts
- [ ] Appends the new recipient to `invoice_config.yaml`
- [ ] Asks if this should become the new default recipient
- [ ] Prints confirmation: `✓ Recipient "key" added to invoice_config.yaml`

**Dependencies:** Story 7.1, Story 1.3

---

### Story 7.6: Recipient Deletion Subcommand
**As a** user,
**I want** to run `invoice recipient delete <key>` to remove a client profile,
**So that** I can clean up old clients.

**Acceptance Criteria:**
- [ ] Prompts for confirmation: `Delete recipient "key" (Company Name)? (y/N)`
- [ ] On "y", removes the recipient from `invoice_config.yaml` and prints `✓ Recipient "key" deleted from invoice_config.yaml`
- [ ] On "N" or Enter, aborts without changes
- [ ] If the recipient is the only remaining one, refuses: `Cannot delete — at least one recipient must exist.`
- [ ] If deleting the `default_recipient`, prompts user to select a new default from remaining recipients
- [ ] Unknown key prints error and exits with non-zero code

**Dependencies:** Story 7.1, Story 1.3

---

## Epic 8: Multi-Currency Support
> Allow different currencies per preset and validate consistency within a single invoice.

### Story 8.1: Per-Preset Currency Override
**As a** user,
**I want** each preset to optionally specify its own currency,
**So that** I can have presets for clients who pay in different currencies.

**Acceptance Criteria:**
- [ ] Each preset may include an optional `currency` field (ISO 4217 code)
- [ ] If `currency` is absent, the preset uses `defaults.currency`
- [ ] `defaults.currency` accepts any ISO 4217 code (EUR, USD, GBP, CZK, UAH, etc.)
- [ ] The PDF renders the correct currency code in rate and amount columns
- [ ] Backwards compatible: existing configs without per-preset currency work unchanged

**Dependencies:** Story 1.1

---

### Story 8.2: Mixed-Currency Validation
**As a** user,
**I want** the app to reject invoices with mixed currencies,
**So that** I don't accidentally combine EUR and USD line items on one invoice.

**Acceptance Criteria:**
- [ ] At generation time, validates all line items share the same currency
- [ ] If currencies differ, prints error listing the conflicting currencies and exits with non-zero code
- [ ] Applies to both interactive and non-interactive modes
- [ ] Single-item invoices always pass this check

**Dependencies:** Story 8.1

---

## Epic 9: Tax / VAT Auto-Calculation
> Optional tax rate per line item with automatic calculation and updated PDF layout.

### Story 9.1: Tax Rate Config and Defaults
**As a** user,
**I want** each preset to optionally include a default tax rate,
**So that** tax is calculated automatically for clients that require VAT.

**Acceptance Criteria:**
- [ ] Each preset may include an optional `tax_rate` field (percentage, e.g. `21.0`)
- [ ] If `tax_rate` is absent or `0`, the line item has no tax (v1.0 behavior)
- [ ] Tax rate is stored as a percentage, not a decimal fraction
- [ ] Config remains backwards-compatible: missing `tax_rate` means no tax

**Dependencies:** Story 1.1

---

### Story 9.2: Tax Rate Override in Interactive Flow
**As a** user,
**I want** to override the tax rate when entering line item details,
**So that** I can adjust tax for special cases without changing the config.

**Acceptance Criteria:**
- [ ] After the rate prompt (Story 3.4), prompts for tax rate with the preset's default shown in brackets
- [ ] Enter accepts the default; entering `0` means no tax for this item
- [ ] Tax prompt only appears when the preset has a non-zero `tax_rate` — presets without `tax_rate` skip the prompt entirely
- [ ] Tax amount is computed as `amount * tax_rate / 100`, rounded to 2 decimal places

**Dependencies:** Story 9.1, Story 3.4

---

### Story 9.3: Tax Rate in Non-Interactive Mode
**As a** user,
**I want** to specify `tax_rate` in the `--items` JSON array,
**So that** I can control tax per line item in scripted generation.

**Acceptance Criteria:**
- [ ] Each item in `--items` JSON may include an optional `tax_rate` field
- [ ] If omitted, uses the preset's `tax_rate` (which itself defaults to 0)
- [ ] `tax_rate` must be >= 0; negative values produce an error

**Dependencies:** Story 9.1, Story 6.3

---

### Story 9.4: PDF Layout with Tax Columns
**As a** user,
**I want** the PDF to show tax breakdown when any line item has tax,
**So that** the invoice meets VAT requirements.

**Acceptance Criteria:**
- [ ] When at least one line item has a non-zero tax rate, the table gains **Tax (%)** and **Tax Amount** columns
- [ ] Rows with zero tax show "–" in the tax columns
- [ ] The single TOTAL row is replaced by three rows: **Subtotal** (sum of net amounts), **Tax** (sum of tax amounts), **Total** (subtotal + tax)
- [ ] When no line items have tax, the PDF renders identically to v1.0 (no tax columns, single TOTAL row)
- [ ] Tax amounts use the same currency formatting as other amounts

**Dependencies:** Story 9.1, Story 4.2

---

## Epic 10: Custom PDF Branding
> Optional config section to customize the visual appearance of generated PDFs.

### Story 10.1: Branding Config Section
**As a** user,
**I want** to configure logo, accent color, font, and footer text in my config,
**So that** my invoices match my personal brand.

**Acceptance Criteria:**
- [ ] Config supports an optional `branding` section with fields: `logo`, `accent_color`, `font`, `footer_text`
- [ ] All fields are optional — sensible defaults (matching v1.0 styling) apply if absent
- [ ] Config remains backwards-compatible: missing `branding` section produces v1.0 styling
- [ ] Validation: `accent_color` must be a valid hex color if provided; invalid value prints warning and falls back to default

**Dependencies:** Story 1.1

---

### Story 10.2: Logo in PDF Header
**As a** user,
**I want** to place my logo in the invoice header,
**So that** invoices look professional and branded.

**Acceptance Criteria:**
- [ ] `logo` field accepts a file path relative to the config file location
- [ ] Supported formats: PNG, JPEG
- [ ] Logo is scaled to fit the header area without distorting aspect ratio
- [ ] If logo file is not found, prints warning and generates PDF without logo (does not fail)

**Dependencies:** Story 10.1, Story 4.2

---

### Story 10.3: Accent Color, Font, and Footer
**As a** user,
**I want** to customize the accent color, font, and footer text,
**So that** invoices are visually consistent with my brand.

**Acceptance Criteria:**
- [ ] `accent_color` (hex) is applied to the "INVOICE" heading, table header row, and horizontal rule lines
- [ ] `font` overrides the default font family; falls back to default if the specified font is unavailable (note: available fonts depend on what Typst can discover — bundled + system fonts)
- [ ] `footer_text` replaces the default footer message; empty string omits the footer entirely
- [ ] Changes are purely visual — no impact on data content or layout structure

**Dependencies:** Story 10.1, Story 4.2

---

## Epic 11: v2.0 Error Handling & Backwards Compatibility
> Cross-cutting robustness for new v2.0 features and seamless migration from v1.0 configs.

### Story 11.1: v1.0 Config Backwards Compatibility
**As a** user upgrading from v1.0,
**I want** my existing config file to work without changes,
**So that** I can upgrade the binary without a migration step.

**Acceptance Criteria:**
- [ ] v2.0 reads and works with unmodified v1.0 config files
- [ ] Single-`recipient` structure is treated as a single-profile list automatically
- [ ] Missing `tax_rate`, `branding`, `recipients`, and per-preset `currency` fields are handled with sensible defaults
- [ ] Running `invoice` (no subcommand) on a v1.0 config produces the same result as v1.0
- [ ] No deprecation warnings for v1.0 config format

**Dependencies:** Story 7.1, Story 8.1, Story 9.1, Story 10.1

---

### Story 11.2: CLI Error Messages for v2.0 Commands (partial — Epic 6 criteria met)
**As a** user,
**I want** clear error messages for all v2.0 CLI mistakes,
**So that** I can fix my command without guessing.

**Acceptance Criteria:**
- [ ] Unknown subcommand → prints usage help and exits with non-zero code
- [ ] Missing required flags in `generate` → lists missing flags, prints usage for `generate`, exits non-zero
- [ ] Malformed `--items` JSON → prints parse error, exits non-zero
- [ ] Unknown preset key in `--items` → prints error naming the unknown key, exits non-zero
- [ ] Mixed currencies → prints error listing conflicting currencies, exits non-zero
- [ ] Unknown `--client` key → prints error listing available client profiles, exits non-zero
- [ ] Deleting last preset → `Cannot delete — at least one preset must exist.`
- [ ] Deleting last recipient → `Cannot delete — at least one recipient must exist.`

**Dependencies:** Story 6.1

---

### Story 11.3: Setup Wizard Update for Multi-Recipient Format
**As a** user,
**I want** the setup wizard to create the v2.0 multi-recipient config format,
**So that** new installations start with the modern structure.

**Acceptance Criteria:**
- [ ] During first-run setup, recipient collection (Story 2.3) creates a `recipients` array with a single entry
- [ ] Prompts for a short `key` for the recipient (or derives from company name)
- [ ] Sets `default_recipient` to that key
- [ ] After initial setup, user can add more recipients via `invoice recipient add` (Story 7.5)
- [ ] Setup wizard still works identically for the single-recipient case

**Dependencies:** Story 7.1, Story 2.3

---

## v2.0 Resolved Gaps & Ambiguities

| # | SRS Section | Issue | Resolution |
|---|-------------|-------|------------|
| 1 | §2.1 — Single-item rate override | No `--rate` flag for `--preset --days` mode — can the user override rate? | **Use `--items` for rate overrides.** Single-item mode always uses `default_rate`. |
| 2 | §3.1 — Recipient management | No CLI commands to add/edit/delete recipients | **Added `invoice recipient list/add/delete` subcommands** (Stories 7.4–7.6). |
| 3 | §4 — Currency on rate override | Does `--items` rate override change currency? | **No.** Rate override changes only the number; currency always comes from preset/default. |
| 4 | §5.2 — Tax prompt for tax-free presets | Should tax prompt appear when preset has no `tax_rate`? | **No.** Tax prompt only appears when preset has a non-zero `tax_rate`. |
| 5 | §6.2 — Font availability | Typst has its own font loading — which fonts are available? | **Implementation detail.** Use fonts discoverable by Typst (bundled + system). Fall back to default if unavailable. |
| 6 | §2.1 — Overwrite in CLI mode | Should non-interactive mode prompt for overwrite? | **Silently overwrite.** Non-interactive mode always overwrites existing files. |

---

## v2.0 Dependency Map

**Critical path** (longest chain):

```
6.1 (Subcommand routing) ✅
  → 6.2 (Single-item generate) ✅
    → 6.3 (Multi-item generate) ✅
      → 9.3 (Tax in CLI mode)
```

**Secondary chains:**

```
7.1 (Multi-recipient config) → 7.2 (Interactive selection) → 7.3 (CLI flag)
7.1 → 7.4 (Recipient list) + 7.5 (Recipient add) + 7.6 (Recipient delete)
7.1 → 11.1 (Backwards compat) → 11.3 (Setup wizard update)

9.1 (Tax config) → 9.2 (Interactive tax override) → 9.4 (PDF tax layout)
9.1 → 9.3 (CLI tax)

10.1 (Branding config) → 10.2 (Logo) + 10.3 (Color/font/footer)

8.1 (Per-preset currency) → 8.2 (Mixed-currency validation)
```

**Recommended sprint ordering:**

1. **Sprint 5** ✅: Stories 6.1, 6.4, 6.5 (subcommand scaffold + preset management)
2. **Sprint 6** ✅: Stories 6.2, 6.3 (non-interactive generation)
3. **Sprint 7**: Stories 7.1, 7.2, 7.3, 7.4, 7.5, 7.6, 11.1 (multi-recipient + backwards compat)
4. **Sprint 8**: Stories 8.1, 8.2 (multi-currency)
5. **Sprint 9**: Stories 9.1, 9.2, 9.3, 9.4 (tax/VAT)
6. **Sprint 10**: Stories 10.1, 10.2, 10.3 (branding)
7. **Sprint 11**: Stories 11.2, 11.3 (error handling polish + setup wizard update)
