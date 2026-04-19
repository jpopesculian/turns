# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Commands

- Build: `cargo build`
- Run all tests: `cargo test`
- Run a single test: `cargo test <name>` (e.g. `cargo test display_three_pi_over_two`)
- Run tests under `no_std`: `cargo test --no-default-features --features libm`
- Lint: `cargo clippy --all-targets`
- Format: `cargo fmt` (check-only: `cargo fmt --check`)

Both feature configurations (default `std` and `--no-default-features --features libm`) must build and pass tests; CI exercises both as well as clippy and fmt checks.

## Architecture

Single-file crate (`src/lib.rs`) exporting `Angle<T>(pub T)`, a fixed-point angle where the full range of an unsigned integer `T` maps to one turn (2π). Integer wraparound _is_ the modular arithmetic — this is the core design idea; prefer wrapping arithmetic over explicit `% TAU`.

Key invariants and conventions:

- **Raw encoding.** The raw value `self.0` encodes angle as `raw / 2^N · 2π`, so `T::max_value() / 2 + 1` equals π. `Angle::<T>::PI`, `FRAC_PI_2`, etc. are defined via the `impl_consts!` macro and derived from this formula.
- **Two layers of impls.** Generic impls (`impl<T: SomeTrait> Angle<T>`) provide the bulk of behavior; the `impl_consts!` macro specializes per-type constants (`ZERO`, `PI`, `FRAC_PI_*`) for `u8`, `u16`, `u32`, `u64`, `u128`, `usize`. `Display` is generic over `T: PrimInt + Display`, not per-type.
- **Operator semantics.** `Add`/`Sub`/`Neg`/`Mul<T>` use _wrapping_ arithmetic (`WrappingAdd`, etc.) so `2π` wraps to `0`. `Div<T>`/`Rem<T>` are plain integer ops on the raw value. `checked_mul`/`checked_div`/`checked_rem` surface overflow/div-by-zero as `Option`.
- **Fractional representation.** `to_frac()` returns `(num, den)` reduced such that `angle = num·π/den` (den is always a power of two). `from_frac(num, den)` is the inverse and is exact for `den` that divides π's raw value (any power of two ≤ `2^(N-1)`) — which covers every `(num, den)` `to_frac` produces, guaranteeing round-trip. `Display` is implemented on top of `to_frac`.
- **Float conversions.** `from_radians`/`to_radians`/`from_degrees`/`to_degrees`/`from_atan2` are generic over `F: Float + FloatConst + Euclid`. Non-finite inputs coerce to the zero angle (do not panic).
- **Cross-width casts.** `cast::<U>()` widens by shifting into the top bits and narrows by keeping top bits — the angle is preserved up to the target's resolution.

## no_std

`#![cfg_attr(not(feature = "std"), no_std)]`. Transcendentals require either the `std` or `libm` feature (both re-exported through `num-traits`). Tests live inside the lib's no_std scope, so the test module uses `extern crate alloc; use alloc::format;` — any new test that needs `format!` or heap allocation must go through `alloc`.

## Version control

Repo uses `jujutsu` (`jj`). Do not squash, rebase, or push unless explicitly asked.
