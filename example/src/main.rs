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
