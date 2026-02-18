# Repository Guidelines

## Project Structure & Module Organization
- `src/` contains the Rust application and library code.
- `src/main.rs` is the executable entrypoint for layout optimization.
- `src/lib.rs` exports core modules: `algorithms`, `keyboard_layout`, and `n_gram`.
- `src/algorithms/genetic.rs` implements the genetic optimizer.
- `src/keyboard_layout/` holds logical/physical layout models and hand/finger mapping.
- `python/dataset.py` handles dataset download and preprocessing.
- `data/` stores generated inputs and SQLite n-gram DB files (for example `ja_en.db`).

## Build, Test, and Development Commands
- `cargo run --release` runs the optimizer with production performance.
- `cargo test` runs Rust unit tests (currently centered on n-gram generation/DB behavior).
- `cargo check` performs fast compile-time validation during iteration.
- `cargo fmt` formats Rust code using `rustfmt` defaults.
- `poetry install` installs Python dependencies.
- `poetry run python python/dataset.py init --data-dir data` fetches raw corpora.
- `poetry run python python/dataset.py --data-dir data` builds cleaned `ja.txt`/`en.txt`.

## Coding Style & Naming Conventions
- Rust: 4-space indentation, `snake_case` for functions/modules, `PascalCase` for types, `SCREAMING_SNAKE_CASE` for constants.
- Keep modules focused: algorithm logic in `src/algorithms/`, layout/domain logic in `src/keyboard_layout/`.
- Prefer small, explicit functions and avoid hidden side effects.
- Run `cargo fmt` before committing; use `cargo clippy` when adding non-trivial logic.

## Testing Guidelines
- Add unit tests close to implementation using `#[cfg(test)] mod tests` (see `src/n_gram.rs`).
- Name tests by behavior, e.g. `test_generate_n_grams`.
- Ensure tests clean up temporary files/DBs they create.
- Run `cargo test` locally before opening a PR.

## Commit & Pull Request Guidelines
- Follow the existing commit pattern: `<type>: <concise description>` (for example `refactor: optimize layout mutation logic`).
- Use imperative, scope-specific summaries; keep unrelated changes in separate commits.
- PRs should include: purpose, key implementation notes, commands run (`cargo test`, dataset steps if relevant), and linked issues.
- Include sample output or screenshots only when behavior or reporting format changes.

## Security & Configuration Tips
- Keep credentials out of git. Files under `python/.env/` (for example tokens/certs) are local-only and must not be committed.
- Treat `data/*.db` and large raw text files as generated artifacts unless a reviewer explicitly requests them.
