# MyTime

Rust desktop time tracker backed by SQLite.

## Run

```bash
cargo run
```

The app reads and writes `time_tracker.db` in the project root. Existing rows in the original `time_entry` table are preserved.

## Features

- Manual time entry
- Realtime start/end tracking
- Edit and delete records
- Day view and recent records
- Week, month, and year statistics

## Generate Test Data

```bash
cargo run --bin generate_test_data -- --days 30
```

Use `--no-clear` to append test data instead of clearing existing records first.
