# User Stories — Invoice Generator CLI

## Summary
- **Epics**: 5
- **Total User Stories**: 18 (11 completed ✅, 7 remaining)
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

### Story 3.2: Select Preset for Line Item
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

### Story 3.3: Create New Preset Inline
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

### Story 3.4: Enter Line Item Details
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

### Story 3.5: Support Multiple Line Items
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

### Story 3.6: Display Invoice Summary and Confirm
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

### Story 4.1: Compute Invoice Number and Dates
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

### Story 4.2: Render PDF Layout
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

### Story 4.3: Save PDF with Correct Filename
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

### Story 5.1: Validate All Numeric Inputs
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
| YAML config | `serde` + `serde_yaml` |
| Interactive prompts | `inquire` |
| PDF generation | `genpdf` or `printpdf` |
| Date handling | `time` |

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

1. **Sprint 1**: Stories 1.1, 1.2, 1.3 (config foundation) + Rust project scaffolding
2. **Sprint 2**: Stories 2.1–2.7 (full setup wizard)
3. **Sprint 3**: Stories 3.1, 3.2, 3.4, 3.5, 3.6 (core invoice flow) + 5.1 (validation)
4. **Sprint 4**: Stories 4.1, 4.2, 4.3 (PDF generation) + 3.3 (inline preset creation)
