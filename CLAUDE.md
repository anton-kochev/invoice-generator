# Project: Invoice Generator CLI

## Tech Stack
- Language: Rust (edition 2024)
- CLI framework: clap (derive)
- Serialization: serde, serde_json, yaml_serde (serde_yaml)
- Interactive prompts: inquire
- PDF generation: typst, typst-kit, typst-pdf, comemo
- Date handling: time
- Error handling: thiserror
- Testing: built-in test framework, tempfile for isolation

## Commands
- `cargo build`: Build the project
- `cargo test`: Run all unit tests
- `cargo run`: Run the interactive invoice flow (config → setup → invoice → PDF)
- `cargo run -- generate --month 3 --year 2026 --preset dev --days 10`: Non-interactive single-item generation
- `cargo run -- preset list|delete <key>`: Manage presets
- `cargo run -- recipient list|add|delete <key>`: Manage recipients

## Code Conventions
- AAA comments (`// Arrange`, `// Act`, `// Assert`) in all test functions
- Tests use synthetic/fake data, never real personal information
- `#[serde(skip_serializing_if)]` on Option fields to avoid `null` in YAML output
- Field aliases via `#[serde(alias)]` for user convenience (e.g. `bic` → `bic_swift`)
- AppError variant per error category (thiserror, `src/error.rs`)
- Prompter trait for testability — `InquirePrompter` (real) / `MockPrompter` (tests)

## Architecture
- `src/main.rs` — entry point, clap CLI parsing, dispatches to interactive or subcommand handlers
- `src/error.rs` — AppError enum with variants for config, setup, invoice, PDF, preset, recipient, template, and locale errors
- `src/config/` — YAML config: types (Config, Sender, Recipient, PaymentMethod, Preset, Defaults, Branding, TemplateKey), loader, validator, writer
- `src/cli/` — clap subcommands: generate (non-interactive), preset list/delete, recipient list/add/delete, interactive flow
- `src/setup/` — interactive setup wizard: sender, recipient, payment, presets, defaults, prompter
- `src/invoice/` — invoice generation: line items, period, currency, preset selection/creation, summary, display
- `src/locale.rs` — Locale enum (en-US, en-GB, de-DE, fr-FR, cs-CZ, uk-UA) with date/number formatting
- `src/pdf/` — PDF output via typst: data mapping, typst world, compilation, 5 built-in templates
- `docs/` — SRS (v1, v2, v3) and user stories

## Workflow
- Story-by-story development following `docs/user-stories-invoice-generator.md`
- TDD: write tests first, then implement
- Never stage or commit the `.claude/` directory
