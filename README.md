# Envro

A crate to load environment variables from a .env file into the process environment variables

### Getting started

```bash
cargo add envro
```

```rust
use std::env;

use envro::*;

fn main() {
    let env_file = env::current_dir().unwrap().join(".env-sample");

    // load env vars and return them
    let env_vars = load_dotenv(&env_file).unwrap();
    println!("---");
    println!(">> vars from .env file");
    println!("{:#?}", &env_vars);

    // load env vars into env::vars
    load_dotenv_in_env_vars(&env_file).unwrap();
    println!("---");
    println!(">> from env::vars()");
    for (key, value) in env::vars() {
        if !env_vars.contains_key(&key) {
            continue;
        }
        println!("{key}: {value}");
    }
}
```

## Test

Since env vars are been written, tests need to be sync

```bash
cargo test -- --test-threads=1
```

## TODO

- check .evn file contains vars only once
- support multiline string values
- values validation
- github action publish
  - publish crate - see https://github.com/googleapis/release-please
    - run cz bump on CI, create release commit, create github release, cargo publish
      - handle pre release (-dev, -beta ...)
- coerce env vars to types
- test, code coverage

---

## LICENSE

MIT License

Copyright (c) 2024 Simone Sanfratello

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
