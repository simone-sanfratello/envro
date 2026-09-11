# install local tooling and git hooks
setup:
    ./scripts/setup.sh

# update deps, format, and run tests
maintenance:
    ./scripts/maintenance.sh

# run the tests (same as pre-commit)
test:
    cargo test -- --test-threads=1

# run the tests with coverage
test-coverage:
    cargo tarpaulin --tests --fail-under 100 --exclude-files 'target/*'

# format sources and auto-fix clippy lints
format:
    cargo fmt
    cargo clippy --all-targets --all-features --fix --allow-dirty --allow-staged -- -D warnings

# run clippy (check only)
lint:
    cargo clippy --all-targets --all-features -- -D warnings

# build the program
build:
    cargo build

# build the program with release
build-release:
    cargo build --release

# run the program with cargo-watch
watch :
    cargo watch -x test

# fix all clippy warnings
fix: 
    cargo fix --allow-dirty --allow-staged

# release: sync main, verify, bump (commitizen), push tags, publish
release:
    ./scripts/release.sh



