# Changelog

## Unreleased

## 2026-09-01

### Changed

- Addressed various regressions from the recent merge with rust-analyzer
- When verus-analyzer is configured to use cargo-verus *and* to only
  verify the current module/file, run with `cargo verus focus`.

## 2026-08-22

### Added

- Improve Go-To-Definition support
- Add scanning support for loop invariant, ensures, and decreases clauses;
  assert-by requires; assert-forall implies conclusions
- Improve inference for Verus's triple-or and triple-and operators.
- Add clearer configuration setting for controlling which code is verified when a file is saved.
- Support empty requires, ensures, etc. clauses

### Changed

- Synchronized with rust-analyzer at `ece721d6cd`.

## 2026-07-29

### Changed

- Updated the Rust version to 1.97.1.

## 2026-03-23

### Added

- Added support for the Verus `final` keyword.

### Changed

- Updated the Rust version to 1.94.0.

## 2026-03-13

### Added

- Added the Fold Proof Block command.

## 2026-03-06

### Added

- Allowed use of the Rust `matches!` macro without confusing it with the Verus `matches` keyword.

## 2025-10-20

### Added

- Added filtering that reports verifier errors from the module currently being edited.

### Fixed

- Fixed argument handling for `cargo verus`.

## 2025-10-03

### Added

- Added support for additional `cargo verus` options.

## 2025-09-29

### Added

- Added `assume_specification` support for constants.

## 2025-09-03

### Added

- Added a setting to choose between direct Verus and `cargo verus` invocation.
