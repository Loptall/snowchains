# Changelog

## [0.4.2] - 2020-08-20Z

### Changed

- Now `RetrieveTestCases` for AtCoder proceeds when encountered scraping errors.

### Fixed

- Improved the parser for `RetrieveTestCases`.

## [0.4.1] - 2020-08-18Z

### Fixed

- Improved `RetrieveTestCases` for AtCoder. ([#76](https://github.com/qryxip/snowchains/pull/76))

## [0.4.0] - 2020-08-16Z

### Changed

- Made `RetrieveTestCasesOutcomeProblem.screen_name` `Option<String>`.
- Made `SubmitOutcome.problem_screen_name` `Option<String>`.

## [0.3.1] - 2020-08-07Z

### Fixed

- Fixed the parser for AtCoder submissions. ([#71](https://github.com/qryxip/snowchains/pull/71))

## [0.3.0] - 2020-08-06Z

### Added

- Added `RetrieveSubmissionSummaries` action for AtCoder.

## [0.2.0] - 2020-08-05Z

### Changed

- Added `names` argument to `BatchTestSuite::load_test_cases`.
