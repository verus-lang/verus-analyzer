# Changelog

## [Unreleased]

## 2026-03-06

### Added
- Allow use of Rust macro `matches!` by avoiding confusion with Verus `matches` keyword (#72)

### Changed
- Updated to Rust version to 1.93.1

## 2026-02-03

### Changed
- Updated to Rust version to 1.93.0


## 2026-01-12

### Changed
- Updated to Rust version to 1.92.0
- Documentation improvements


## 2025-11-18 

### Changed
- Updated to Rust version to 1.91.0
- Improved error message when Rust toolchain validation fails


## 2025-11-06

### Added
- Add logic to try to locate the default cargo home directory


## 2025-10-20

### Added
- Setting for enabling/disabling the feature that tries to only report errors in the file that's currently being edited.

### Fixed
- Various bugs related to `cargo verus` arugments


## 2025-10-03

### Added
- Support for more `cargo verus` options


## 2025-09-29

### Added
- Support for `assume_specification` for consts (see verus-lang/verus#1825)


## 2025-09-03

### Added
- A config flag to enable toggling between `cargo verus` and direct `verus` invocation


