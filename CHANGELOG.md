## 1.9.0 (2026-03-23)

## Features :tada:

- Support for `ssh-agent daemon` to have the SSH agent run in the background.

## Fixes :bug:

- Improve invite acceptance flow.
- Allow to print SSH Key keys with `private key` and `public key` field specifiers.

## Other

- Updated dependencies.
- Fixed docs typos.

## 1.8.0 (2026-03-23)

## Features :tada:

- Support for Linux DBUS integration.

## Other

- Updated dependencies.

## 1.7.0 (2026-03-18)

## Features :tada:

- Add support for PKCS#8 SSH keys in the SSH agent.

## Fixes :bug:

- `pass-cli run` with env files now doesn't drop values that are not pass secret references.

## Other

- Updated dependencies.
- Improved local secret key storage in system keyrings.

## 1.6.1 (2026-03-10)

## Other

- Fixed codesign of macOS binaries.

## 1.6.0 (2026-03-09)

## Features :tada:

- CLI flag completion for Bash/Zsh/Fish (Thanks to @vichid for the contribution)

## Fixes :bug:

- Fix support for RSA4096 keys in the SSH agent

## 1.5.2 (2026-02-23)

## Fixes :bug:

- Add missing DLL for Windows installs

## 1.5.1 (2026-02-19)

## Other

- Improved behaviour on session expiration

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
