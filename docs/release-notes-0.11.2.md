# sqlglot-rust 0.11.2

## Maintenance

- Updated the locked development dependency `rustix` from 1.1.5 to 1.1.6 to
  address [RUSTSEC-2026-0486](https://rustsec.org/advisories/RUSTSEC-2026-0486).
  `rustix` is used through the `tempfile` development dependency and is not a
  runtime dependency of the published crate.
