# centsh — Terminal budgeting with live charts and auto-budgets
One-screen TUI to log income/expenses, see category spend and cashflow charts, and get monthly budget suggestions from your own history.

![License](https://img.shields.io/badge/license-MIT-green)

## Table of Contents
- [Overview / Features](#overview--features)
- [Screenshots / Demo](#screenshots--demo)
- [Installation](#installation)
- [Usage Examples](#usage-examples)
- [Configuration](#configuration)
- [Project Structure](#project-structure)
- [Roadmap](#roadmap)
- [Contributing](#contributing)
- [License](#license)
- [Acknowledgements](#acknowledgements)
- [Changelog](#changelog)

## Overview / Features
- Keyboard-first TUI (tabs for Overview, Transactions, Budgets) with live charts for category spend and monthly cashflow.
- Quick-add forms for transactions and budgets, plus auto-budget suggestions from the last 90 days of spending.
- Local JSON storage in your OS data directory (macOS: `~/Library/Application Support/centsh/ledger.json`—XDG on Linux).
- Sensible sample data on first run so you see charts immediately.

## Screenshots / Demo
- Run `cargo run` and you’ll see:
  - Overview: income/spend/net, budgets progress, category bar chart, cashflow line chart.
  - Transactions: sortable table of recent entries.
  - Budgets: limits per category plus auto-budget hints (toggle with `g`).
- Tip: capture a short demo GIF with `asciinema rec` or `terminalizer` and drop it here once ready.

## Installation
Prerequisites: Rust stable (via `rustup`). macOS/Linux terminal.

Local dev run:
```bash
cargo run
```

Install as the `centsh` command from source:
```bash
cargo install --path .
centsh
```

Planned Homebrew tap (after publishing a release):
```bash
brew tap Thatkidtk/tap
brew install thatkidtk/tap/centsh
```

## Usage Examples
- Launch: `centsh`
- Normal mode keys: `a` add transaction, `b` add/update budget, `h/l` switch tabs, `s` save, `g` toggle auto-budget hints, `r` reload, `q` quit.
- Form mode: type to edit fields, `Tab`/`Shift+Tab` to move, `Enter` to advance/submit, `Esc` to cancel.
- Amount convention: expenses are positive numbers; income is negative. Net = income − spending.
- Dates: `YYYY-MM-DD` (defaults to today if left blank).

## Configuration
- Storage path: macOS `~/Library/Application Support/centsh/ledger.json`; Linux/other XDG data dir. Want a custom path? (Future idea: add `CENTSHPATH` environment variable.)
- Budgets are monthly per category; auto-budget looks at last 90 days spend per category, averages monthly, adds 10% buffer.

## Project Structure
```
src/
  main.rs       # TUI + input handling
  models.rs     # Ledger, budgets, transactions, auto-budget logic
  storage.rs    # JSON persistence in OS data dir
Cargo.toml      # crate/deps metadata
```

## Roadmap
- Editing/deleting transactions; CSV import/export.
- Configurable data path and theming.
- Alerts/envelopes when nearing limits; recurring transactions and goals.
- CI (lint/test) and Homebrew release automation.

## Contributing
Issues/PRs welcome. Please run `cargo fmt && cargo clippy --all-targets -- -D warnings && cargo test` before opening a PR. For feature ideas, describe the workflow/use-case first.

## License
MIT. See [LICENSE](LICENSE).

## Acknowledgements
- Built with [ratatui](https://github.com/ratatui-org/ratatui), [crossterm](https://github.com/crossterm-rs/crossterm), and [chrono](https://github.com/chronotope/chrono).

## Changelog
- Track releases via Git tags (e.g., `v0.1.0`). Add a `CHANGELOG.md` when versions start shipping via Homebrew.
