## [0.5.1] - 2020-08-22Z

### Changed

- Now `retrieve testcases` for AtCoder proceeds when encountered scraping errors.

### Fixed

- Improved the parser for `retrieve testcases`.
- Fixed a bug that [`package.dhall`](https://github.com/qryxip/snowchains/blob/master/resources/config/schema/Snowchains/package.dhall) cannot be resolved unless [dhall-haskell](https://github.com/dhall-lang/dhall-haskell) resolves it.

## [0.5.0] - 2020-08-18Z

### Changed

- Modified JSON output of `retrieve testcases` and `submit`. ([#74](https://github.com/qryxip/snowchains/pull/74))

### Fixed

- Improved the parser for `retrieve testcases`. ([#76](https://github.com/qryxip/snowchains/pull/76))

## [0.4.2] - 2020-08-06Z

### Added

- Added `retrieve submission-summaries` command.

### Fixed

- Fixed the problem that `snowchains` did not work at all.

## [0.4.1] - 2020-08-05Z

### Added

- Added `--testcases` option to `judge` command and `submit` command.

## [0.4.0] - 2020-08-02Z

### Changed

* Almost everything.

## [0.3.3] - 2020-05-01Z

### Fixed

* On 2020-05-01, AtCoder updated the submission pages and broke utility tools including `snowchains`. Now `snowchains` follows the update.

## [0.3.2] - 2020-04-26Z

### Fixed

* Fixed build on Rust 1.42.0.
* Fixed the parser for yukicoder.

## [0.3.1] - 2020-04-21Z

### Removed

* Dropped the support for `x86_64-pc-windows-gnu`.

### Fixed

* Fixed the parser for Codeforces.

## [0.3.0] - 2020-01-29Z

### Added

* `-C|--colorize` option, which equivalent to `--color always`
* `--mode` option to `judge`, `retrieve submissions`, and `submit`.
* `--release` option to `judge`, which equivalent to `--mode release`.
* `--debug` option to `submit`, which equivalent to `--mode debug`.
* `.command_line_arguments`, `.config`, `.base_directory`, and `.target` to the result JSON.
* `--output` option and `--json` flag to all of the subcommands.
* `--no-save` option to `retrieve testcases` and `retrieve submissions`.
* `--fetch-all` flag to `retrieve submissions` command.
* `session.retry` fields.
* `shell._` now accept `{ runner = "..", extension = ".." }` to run temporary script files.

### Changed

* Rename `download` command to `retrive testcases` (`download` remains as an alias)
* Rename `restore` command to `retrive submissions`
* Rename `list-langs` command to `retrive languages`
* Replace `--only-scraped` with `--full`
* Make `env` field more configurable.
* Now it prints most of the information to stderr, not stdout.
* Move `service`, `contest` and `language` fields of `snowchains.toml` to another file.
* Make it find a executable binary's path with [`which`](https://crates.io/crates/which) crate.
* Now it retrieves and parses `tasks_print` pages to improve the performance.
* Save cookies as JSON files.
* `$SNOWCHAINS_RESULT` to STDIN.

### Removed

* `show _` commands.
* `modify _` commands.

### Fixed

* Fix the condition of enabling ANSI color when `--color auto`.
* Fix a bug where `retrive languages` displays &lt;number of problems&gt; copies of a list.
* Prevent `retrieve languages` from displaying unavailable languages when `problem` is specified.
* Change the default value of `jobs` from `1` to the number of CPUs.

## [0.2.0] - 2019-03-03

### Added

* Codeforces support
* `list-langs` command
* `session.robots` field (default: `true`)

### Changed

* `session.api_tokens` field (required)
* `languages._.language_ids` to `languages._.names` (always required)

### Fixed

* Suppressed the unnecessary newline when printing errors
