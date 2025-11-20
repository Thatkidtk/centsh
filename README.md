# centsh (TUI)

Terminal-first budget tracker with live charts, quick input, and auto-budget hints. Data is stored locally in `~/Library/Application Support/centsh/ledger.json` on macOS (XDG data dir elsewhere).

## Run it locally
- `cargo run` — launches the TUI with sample data if you have none.
- Install the binary for the `centsh` command: `cargo install --path .` then run `centsh`.
- Keys (normal mode): `a` add transaction, `b` add/update budget, `h/l` switch tabs, `s` save, `g` toggle auto-budget hints, `r` reload from disk, `q` quit.
- Form mode (add transaction/budget): type to edit, `Tab`/`Shift+Tab` to move, `Enter` to advance/submit, `Esc` to cancel.

### Data conventions
- Amounts: expenses are positive numbers; income is negative. Net = income − spending.
- Dates: `YYYY-MM-DD` (defaults to today if left blank).
- Budgets: monthly limits by category. The overview shows % consumed for the current month.

### Auto-budget logic
Looks at the last 90 days of spending per category, averages to a monthly number, adds a 10% buffer, and proposes a suggested limit. If you have no history yet, it suggests starter categories (Housing, Food, Savings).

## Shipping on macOS/Homebrew
1) Build a release binary: `cargo build --release` (for universal: `cargo build --release --target aarch64-apple-darwin` and `x86_64-apple-darwin`, then `lipo -create`).
2) Create a versioned Git tag and GitHub release attaching `centsh` binary (or tarball with binary + README).
3) Homebrew tap formula sketch:
   ```ruby
   class Centsh < Formula
     desc "Terminal budgeting app with graphs and auto-budgeting"
     homepage "https://github.com/you/centsh"
     url "https://github.com/you/centsh/releases/download/v0.1.0/centsh.tar.gz"
     sha256 "<replace_with_actual>"
     license "MIT"

     def install
       bin.install "centsh"
     end

     test do
       system "#{bin}/centsh", "--help"
     end
   end
   ```
   Then `brew tap you/tap` and `brew install you/tap/centsh`.

## Ideas to extend
- Editing/deleting transactions, CSV import/export.
- Envelope-style budgets and alerts when nearing limits.
- Multi-wallet accounts, recurring transactions, savings goals.
- Richer charts (stacked categories, cash runway) and configurable themes.
# centsh
