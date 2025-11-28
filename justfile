# run the tests
test:
    cargo test --tests -- --nocapture

# run the tests with coverage
test-coverage:
    cargo tarpaulin --tests --fail-under 100

# update the dependencies to the latest version
deps-update:
    cargo upgrade -i allow && cargo update

# run clippy
format:
    cargo fmt

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
