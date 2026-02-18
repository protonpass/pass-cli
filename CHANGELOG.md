## 1.5.0 (2026-02-18)

## Features :tada:

- Offer `ssh-agent debug` command to debug whether the items of a vault can be used as SSH keys
- Improvements on crypto dependencies

## 1.4.3 (2026-02-09)

## Other

- Renamed internal config variables to avoid clashing with commonly defined variables (`ENVIRONMENT`)

## 1.4.2 (2026-02-04)

## Other

- Improvements on permission handling
- Add modify time to `item view` in JSON output format

## 1.4.1 (2026-01-21)

## Fixes :bug:

- Fixed `pass-cli item update` duplicating custom fields when updating an item

## 1.4.0 (2026-01-20)

## Features :tada:

- Added `pass-cli settings` to set default values for vault and output format

## 1.3.5 (2026-01-12)

## Other

- Make `pass-cli info` capable of printing the output in JSON format

## 1.3.4 (2026-01-12)

## Other

- Improve windows SSH agent messages and documentation

## 1.3.3 (2026-01-09)

## Other

- Offer `--capitalize` alongside `--capitalise` for `password generate passphrase`
- Documentation fixes

## 1.3.2 (2025-12-18)

## Features :tada:

- `ssh-agent`: now supports ssh certificates added via `ssh-add`

### Bug fixes :bug:

- Fix `pass-cli login` command for users created with non-proton accounts

## 1.3.1 (2025-12-17)

### Bug fixes :bug:

- Fix `pass-cli info` command for users created with non-proton accounts

## 1.3.0 (2025-12-16)

### Features :tada:

- Allow `ssh-agent` to create identities based on imported keys

### Bug fixes :bug:

- `pass-cli run` now supports secret references with spaces in the components
- Fix for some users getting errors when running `pass-cli info`

### Other

- Documentation fixes
