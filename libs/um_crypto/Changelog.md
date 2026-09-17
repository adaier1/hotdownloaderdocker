# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.12] - 2025-09-08

### Fixed
- Address clippy warnings
- Reduce mp3 false-positive detection

## [0.1.11] - 2025-09-03

### Changed
- Exclude .idea directory from version control
- Moved to new git hosting.

## [0.1.10] - 2025-05-04

### Changed
- Remove anyhow dependency from kuwo module
- Remove anyhow usage from qmc module

### Fixed
- Fix wasm build process

## [0.1.9] - 2025-03-31

### Changed
- Drop mp1/mp2 format test

## [0.1.8] - 2025-03-31

### Changed
- MP3: sync frame check

## [0.1.7] - 2025-03-31

### Added
- Script to bump all versions

### Fixed
- Support mp3 files with junk data after header

## [0.1.6] - 2025-02-25

### Changed
- Expose KGG db decryption function.
- MKV will have `.mka` extension.

## [0.1.5] - 2025-02-25

### Added
- Add MKV detection.

## [0.1.4] - 2025-02-25

### Changed
- Refactor KGM.

## [0.1.3] - 2025-02-24

### Added
- Support for KGG file format (KGMv5)

## [0.1.2] - 2024-12-15

### Changed
- Detect mp3 file that had multiple ID3 tag.

## [0.1.1] - 2024-12-15

### Added
- QRC file support

## [0.1.0] - 2024-12-15

### Added
- Initial release
