# Changelog

All notable changes to this project will be documented in this file.

## [0.3.0] - 2026-03-25

### 🚀 Features

- Add progress bar using indicatif
- *(install)* Print created hardlinks
- *(tlist)* Add inner/inner_mut to Reader and Writer
- *(error)* Improve error messages for common failure cases
- *(sdat2img)* Validate total_blocks against transfer list header

### 🚜 Refactor

- [**breaking**] Make positional options named in sdat2img and img2sdat
- *(img2sdat)* [**breaking**] Rename -v/--version to --format
- Add ErrorExt trait to reduce map_err boilerplate
- Use get_ref/inner instead of into_parts
- *(error)* Remove #[source] from terminal Error variants

### 📚 Documentation

- *(readme)* Update usage of sdat2img and img2sdat commands

### 🎨 Styling

- *(img2sdat)* Merge split #[arg] attrs into single attribute
- Sort struct fields in img2sdat, sdat2img
- Remove redundant Ok(()) in write_command and img2sdat
- *(error)* Remove redundant "io error:" prefix from Io message
- *(error)* Remove redundant "data" from Alignment message
- *(tlist)* Inline read_line result in header parsing
- Sort error enum variants alphabetically

### 🧪 Testing

- Add BLOCK_USIZE const to reduce casting noise

## [0.2.0] - 2026-03-13

### 🚀 Features

- *(install)* Add -f/--force option

### 🐛 Bug Fixes

- *(install)* Preserve executable extension in hardlinks

### 📚 Documentation

- *(readme)* Update usage of install command

## [0.1.0] - 2026-03-12

### 🚀 Features

- Implement transfer list reading and writing
- Implement sdat2img
- Implement img2sdat
- Add multicall dispatch and install subcommand

### 💼 Other

- Configure release profile
- Add rust-toolchain.toml
- Set rust-version to 1.91

### 📚 Documentation

- *(readme)* Add README.md

### 🎨 Styling

- Run `cargo +nightly fmt`

### ⚙️ Miscellaneous Tasks

- Initial commit
- Add rustfmt.toml
- Setup `cargo-dist`
- Add LICENSE
- Add cliff.toml
