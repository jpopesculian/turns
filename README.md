# turns

[![CI](https://github.com/jpopesculian/turns/actions/workflows/ci.yml/badge.svg)](https://github.com/jpopesculian/turns/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/turns.svg)](https://crates.io/crates/turns)
[![docs.rs](https://docs.rs/turns/badge.svg)](https://docs.rs/turns)

Fixed-point angles modulo 2π, backed by unsigned integers.

`Angle<T>` represents an angle as an unsigned integer where the full range of
`T` maps onto one full turn (2π radians / 360 degrees). Natural integer
overflow provides wraparound at 2π, so modular arithmetic is free.

Type aliases `Angle8`, `Angle16`, `Angle32`, `Angle64`, and `Angle128` cover
the standard widths.

## Usage

```toml
[dependencies]
turns = "0.1"
```

```rust
use turns::Angle8;
use core::f64::consts::PI;

let pi = Angle8::from_radians(PI);
assert_eq!(pi + pi, Angle8::from_radians(0.0_f64));
```

## Features

- `std` *(default)* — enables `num-traits/std`.
- `libm` — enables `num-traits/libm` for transcendentals in `no_std` builds.

For `no_std`:

```toml
[dependencies]
turns = { version = "0.1", default-features = false, features = ["libm"] }
```

## Precision

Float conversions are generic over any `F: Float`. Expect precision loss when
the integer width exceeds the float mantissa (e.g. `Angle128` with `f64` keeps
~53 of 128 bits). Non-finite inputs (`NaN`, infinities) are coerced to zero
rather than panicking.

## License

MIT
