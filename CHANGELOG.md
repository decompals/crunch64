# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.5.3] - 2024-12-15

### Fixed

- Correct version number for Python project.

## [0.5.2] - 2024-12-15

### Added

- Prebuilt binaries for Python 3.13.
- Check Python files with mypy.
- Set MSRV (minimum supported Rust version) to 1.73.

### Changed

- Python 3.9 or later is now required.
  - Mainly due to not being able to automatically check and build for older versions in CI.

## [0.5.1] - 2024-07-25

### Fixed

- Fix a bug where the gzip compressor may output incorrect data when emitting
  "fixed blocks" (which are emitted when compressing high-entropy data).

## [0.5.0] - 2024-06-04

### Added

- Add matching zlib/DEFLATE compression for old gzip versions.

## [0.4.0] - 2024-06-03

### Added

- Add MIO0 compression to CLI.

### Changed

- Speed up compression by 2100%.
- Move CompressionType from library to CLI.

## [0.3.1] - 2024-01-20

### Fixed

- Fix some rare cases where the Yaz0 compressor may append an extra 0 at the end of the compressed data.

## [0.3.0] - 2024-01-19

### Added

- CHANGELOG.md file.

### Changed

- A few code cleanups.

### Fixed

- Functions not accepting `bytearray` objects.

## [0.2.0] - 2023-12-28

### Added

- MIO0 compression and decompression.

## [0.1.1] - 2023-12-16

### Fixed

- CI for release.

## [0.1.0] - 2023-12-16 [YANKED]

### Added

- Yay0 compression and decompression.
- Yaz0 compression and decompression.
- Python bindings.
- C bindings.

[unreleased]: https://github.com/decompals/crunch64/compare/0.5.1...HEAD
[0.5.3]: https://github.com/decompals/crunch64/compare/0.5.2...0.5.3
[0.5.2]: https://github.com/decompals/crunch64/compare/0.5.1...0.5.2
[0.5.1]: https://github.com/decompals/crunch64/compare/0.5.0...0.5.1
[0.5.0]: https://github.com/decompals/crunch64/compare/0.4.0...0.5.0
[0.4.0]: https://github.com/decompals/crunch64/compare/0.3.1...0.4.0
[0.3.1]: https://github.com/decompals/crunch64/compare/0.3.0...0.3.1
[0.3.0]: https://github.com/decompals/crunch64/compare/0.2.0...0.3.0
[0.2.0]: https://github.com/decompals/crunch64/compare/0.1.1...0.2.0
[0.1.1]: https://github.com/decompals/crunch64/compare/0.1.0...0.1.1
[0.1.0]: https://github.com/decompals/crunch64/releases/tag/0.1.0
