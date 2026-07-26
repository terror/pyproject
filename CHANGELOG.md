# Changelog

## [0.2.1](https://github.com/terror/pyproject/releases/tag/0.2.1) - 2026-07-25

### Added

- Add schema completions ([#174](https://github.com/terror/pyproject/pull/174) by [terror](https://github.com/terror))
- Reject empty project people entries ([#180](https://github.com/terror/pyproject/pull/180) by [terror](https://github.com/terror))

### Fixed

- Write tracing output to stderr ([#177](https://github.com/terror/pyproject/pull/177) by [terror](https://github.com/terror))
- Avoid analyzing disabled rules ([#179](https://github.com/terror/pyproject/pull/179) by [terror](https://github.com/terror))

### Misc

- Move validator to schema store ([#175](https://github.com/terror/pyproject/pull/175) by [terror](https://github.com/terror))
- Migrate logging to tracing ([#176](https://github.com/terror/pyproject/pull/176) by [terror](https://github.com/terror))
- Simplify project people rule ([#181](https://github.com/terror/pyproject/pull/181) by [terror](https://github.com/terror))

## [0.2.0](https://github.com/terror/pyproject/releases/tag/0.2.0) - 2026-07-25

### Added

- Validate top-level keys ([#169](https://github.com/terror/pyproject/pull/169) by [terror](https://github.com/terror))
- Validate PEP 794 import names ([#168](https://github.com/terror/pyproject/pull/168) by [terror](https://github.com/terror))
- Validate dependency groups ([#167](https://github.com/terror/pyproject/pull/167) by [terror](https://github.com/terror))
- Expose library interface ([#164](https://github.com/terror/pyproject/pull/164) by [terror](https://github.com/terror))
- Expand schema store tool schemas ([#162](https://github.com/terror/pyproject/pull/162) by [terror](https://github.com/terror))
- Add support for quickfix code actions ([#161](https://github.com/terror/pyproject/pull/161) by [terror](https://github.com/terror))
- Add builtin-based completions ([#159](https://github.com/terror/pyproject/pull/159) by [terror](https://github.com/terror))
- Add build system rule ([#157](https://github.com/terror/pyproject/pull/157) by [terror](https://github.com/terror))
- Add project name normalization rule ([#154](https://github.com/terror/pyproject/pull/154) by [terror](https://github.com/terror))
- Validate project name grammar ([#152](https://github.com/terror/pyproject/pull/152) by [terror](https://github.com/terror))
- Update classifiers list ([#145](https://github.com/terror/pyproject/pull/145) by [terror](https://github.com/terror))

### Fixed

- Fix pypi screenshot url ([#171](https://github.com/terror/pyproject/pull/171) by [terror](https://github.com/terror))
- Allow current project dynamic fields ([#153](https://github.com/terror/pyproject/pull/153) by [terror](https://github.com/terror))

### Misc

- Test PyPI client with mockito ([#170](https://github.com/terror/pyproject/pull/170) by [terror](https://github.com/terror))
- Refactor PyPI version selection ([#166](https://github.com/terror/pyproject/pull/166) by [terror](https://github.com/terror))
- Add custom error type ([#165](https://github.com/terror/pyproject/pull/165) by [terror](https://github.com/terror))
- Extract hover resolver ([#160](https://github.com/terror/pyproject/pull/160) by [terror](https://github.com/terror))
- Extract dependency wrapper ([#158](https://github.com/terror/pyproject/pull/158) by [terror](https://github.com/terror))
- Document language server in readme ([#155](https://github.com/terror/pyproject/pull/155) by [terror](https://github.com/terror))
- Centralize project name regex ([#151](https://github.com/terror/pyproject/pull/151) by [terror](https://github.com/terror))
- Test configured rule severities ([#149](https://github.com/terror/pyproject/pull/149) by [terror](https://github.com/terror))
- Test multiple check diagnostics ([#150](https://github.com/terror/pyproject/pull/150) by [terror](https://github.com/terror))
- Add format integration tests ([#148](https://github.com/terror/pyproject/pull/148) by [terror](https://github.com/terror))
- Add integration test suite ([#147](https://github.com/terror/pyproject/pull/147) by [terror](https://github.com/terror))
- Re-format readme introduction ([#146](https://github.com/terror/pyproject/pull/146) by [terror](https://github.com/terror))
- Implement `Config` from conversions ([#144](https://github.com/terror/pyproject/pull/144) by [terror](https://github.com/terror))
- Remove `PyPiError` type ([#143](https://github.com/terror/pyproject/pull/143) by [terror](https://github.com/terror))
- Reorder trait implementations ([#142](https://github.com/terror/pyproject/pull/142) by [terror](https://github.com/terror))
- Simplify schema pointer mapping ([#141](https://github.com/terror/pyproject/pull/141) by [terror](https://github.com/terror))
- Simplify schema error formatting ([#140](https://github.com/terror/pyproject/pull/140) by [terror](https://github.com/terror))

## [0.1.3](https://github.com/terror/pyproject/releases/tag/0.1.3) - 2026-07-16

### Added

- Update remote schema definitions ([#137](https://github.com/terror/pyproject/pull/137) by [terror](https://github.com/terror))

### Misc

- Sort analyzer tests alphabetically ([#136](https://github.com/terror/pyproject/pull/136) by [terror](https://github.com/terror))
- Use inventory for rule registration ([#134](https://github.com/terror/pyproject/pull/134) by [terror](https://github.com/terror))
- Add documentation to rule definitions ([#131](https://github.com/terror/pyproject/pull/131) by [terror](https://github.com/terror))
- Port over remaining rules to use `define_rule` ([#130](https://github.com/terror/pyproject/pull/130) by [terror](https://github.com/terror))
- Migrate `SemanticRule` to use new rule macro ([#129](https://github.com/terror/pyproject/pull/129) by [terror](https://github.com/terror))
- Migrate `SyntaxRule` to use rule macro ([#127](https://github.com/terror/pyproject/pull/127) by [terror](https://github.com/terror))
- Add macro for rule definitions ([#126](https://github.com/terror/pyproject/pull/126) by [terror](https://github.com/terror))
- Lift out standalone publish script ([#124](https://github.com/terror/pyproject/pull/124) by [terror](https://github.com/terror))

## [0.1.2](https://github.com/terror/pyproject/releases/tag/0.1.2) - 2025-11-27

### Added

- Make upper bound warnings opt-in ([#120](https://github.com/terror/pyproject/pull/120) by [terror](https://github.com/terror))

### Misc

- Add python release workflow ([#122](https://github.com/terror/pyproject/pull/122) by [terror](https://github.com/terror))
- Add python package ([#119](https://github.com/terror/pyproject/pull/119) by [terror](https://github.com/terror))

## [0.1.1](https://github.com/terror/pyproject/releases/tag/0.1.1) - 2025-11-26

### Added

- Add support for context-aware completions ([#112](https://github.com/terror/pyproject/pull/112) by [terror](https://github.com/terror))
- Add rule for validating project optional dependencies ([#108](https://github.com/terror/pyproject/pull/108) by [terror](https://github.com/terror))

### Fixed

- Remove classifiers from allowed dynamic keys ([#110](https://github.com/terror/pyproject/pull/110) by [terror](https://github.com/terror))
- Default to error for unknown project keys ([#109](https://github.com/terror/pyproject/pull/109) by [terror](https://github.com/terror))

### Misc

- Scaffold `pyproject-wasm` crate ([#114](https://github.com/terror/pyproject/pull/114) by [terror](https://github.com/terror))
- Add usage section to readme ([#111](https://github.com/terror/pyproject/pull/111) by [terror](https://github.com/terror))
- Add changelog crate ([#107](https://github.com/terror/pyproject/pull/107) by [terror](https://github.com/terror))
- Use dark theme for readme badges ([#106](https://github.com/terror/pyproject/pull/106) by [terror](https://github.com/terror))

## [0.1.0](https://github.com/terror/pyproject/releases/tag/0.1.0) - 2025-11-25

Initial release 🎉
