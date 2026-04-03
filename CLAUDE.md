# Project: Invoice Generator CLI

## Tech Stack
- Language: Rust (edition 2024)
- Serialization: serde + yaml_serde (serde_yaml)
- Error handling: thiserror
- Testing: built-in test framework, tempfile for isolation

## Commands
- `cargo build`: Build the project
- `cargo test`: Run all unit tests
- `cargo run`: Run from current directory (looks for `invoice_config.yaml`)

## Code Conventions
- AAA comments (`// Arrange`, `// Act`, `// Assert`) in all test functions
- Tests use synthetic/fake data, never real personal information
- `#[serde(skip_serializing_if)]` on Option fields to avoid `null` in YAML output
- Field aliases via `#[serde(alias)]` for user convenience (e.g. `bic` → `bic_swift`)
- AppError variants per error category (ConfigParse, ConfigIo)

## Architecture
- `src/main.rs` — entry point, orchestrates load → validate → flow
- `src/error.rs` — AppError enum (thiserror)
- `src/config/` — config module: types, loader, validator, writer
- `docs/` — SRS and user stories

## Workflow
- Story-by-story development following `docs/user-stories-invoice-generator.md`
- TDD: write tests first, then implement
- Never stage or commit the `.claude/` directory
