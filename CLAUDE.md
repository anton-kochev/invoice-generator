# Project: Invoice Generator CLI

## Tech Stack
- Language: Rust (edition 2024)
- Serialization: serde, serde_json, yaml_serde (serde_yaml)
- Interactive prompts: inquire
- PDF generation: typst, typst-kit, typst-pdf, comemo
- Date handling: time
- Error handling: thiserror
- Testing: built-in test framework, tempfile for isolation

## Commands
- `cargo build`: Build the project
- `cargo test`: Run all unit tests
- `cargo run`: Run the full invoice flow (config → setup → invoice → PDF)

## Code Conventions
- AAA comments (`// Arrange`, `// Act`, `// Assert`) in all test functions
- Tests use synthetic/fake data, never real personal information
- `#[serde(skip_serializing_if)]` on Option fields to avoid `null` in YAML output
- Field aliases via `#[serde(alias)]` for user convenience (e.g. `bic` → `bic_swift`)
- AppError variant per error category (thiserror, `src/error.rs`)
- Prompter trait for testability — `InquirePrompter` (real) / `MockPrompter` (tests)

## Architecture
- `src/main.rs` — entry point, orchestrates load → validate → setup → invoice → PDF
- `src/error.rs` — AppError enum (ConfigParse, ConfigIo, SetupCancelled, InvalidDate, PdfCompile, PdfExport)
- `src/config/` — YAML config: types, loader, validator, writer
- `src/setup/` — interactive setup wizard: sender, recipient, payment, presets, defaults, prompter
- `src/invoice/` — invoice generation: line items, period, preset selection/creation, summary, display
- `src/pdf/` — PDF output via typst: data mapping, typst world, compilation
- `docs/` — SRS and user stories

## Workflow
- Story-by-story development following `docs/user-stories-invoice-generator.md`
- TDD: write tests first, then implement
- Never stage or commit the `.claude/` directory
