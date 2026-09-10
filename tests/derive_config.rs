use envro::{Envro, EnvroConfig, EnvroVars};

#[derive(Debug, Envro, PartialEq)]
struct Config {
    #[envro(from = "APP_NAME", min_len = 3, max_len = 64)]
    app_name: String,

    #[envro(from = "PORT", port)]
    port: u16,

    #[envro(from = "OFFSET", integer)]
    offset: Option<i64>,

    #[envro(from = "FEATURE_X", boolean)]
    feature_x: bool,

    #[envro(from = "DB_POOL_SIZE", positive_integer)]
    db_pool_size: i64,
}

#[test]
fn from_vars_ok() {
    let mut vars = EnvroVars::new();
    vars.insert("APP_NAME".into(), "envro".into());
    vars.insert("PORT".into(), "8080".into());
    vars.insert("OFFSET".into(), "-3".into());
    vars.insert("FEATURE_X".into(), "yes".into());
    vars.insert("DB_POOL_SIZE".into(), "4".into());

    let cfg = Config::from_vars(&vars).unwrap();
    assert_eq!(
        cfg,
        Config {
            app_name: "envro".into(),
            port: 8080,
            offset: Some(-3),
            feature_x: true,
            db_pool_size: 4,
        }
    );
}

#[test]
fn from_vars_optional_missing() {
    let mut vars = EnvroVars::new();
    vars.insert("APP_NAME".into(), "envro".into());
    vars.insert("PORT".into(), "8080".into());
    vars.insert("FEATURE_X".into(), "false".into());
    vars.insert("DB_POOL_SIZE".into(), "1".into());

    let cfg = Config::from_vars(&vars).unwrap();
    assert_eq!(cfg.offset, None);
    assert!(!cfg.feature_x);
}

#[test]
fn from_vars_validation_fails() {
    let mut vars = EnvroVars::new();
    vars.insert("APP_NAME".into(), "ab".into()); // min_len 3
    vars.insert("PORT".into(), "70000".into());
    vars.insert("FEATURE_X".into(), "true".into());
    vars.insert("DB_POOL_SIZE".into(), "1".into());

    let err = Config::from_vars(&vars).unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("APP_NAME"), "{msg}");
    assert!(msg.contains("PORT"), "{msg}");
}

#[test]
fn default_env_key_screaming_snake() {
    #[derive(Envro)]
    struct S {
        #[envro(integer)]
        retry_count: i64,
    }
    let mut vars = EnvroVars::new();
    vars.insert("RETRY_COUNT".into(), "2".into());
    assert_eq!(S::from_vars(&vars).unwrap().retry_count, 2);
}

#[test]
fn from_dotenv_and_schema() {
    let dir = std::env::temp_dir().join(format!("envro-derive-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join(".env");
    std::fs::write(
        &path,
        "APP_NAME=envro\nPORT=8080\nFEATURE_X=true\nDB_POOL_SIZE=4\n",
    )
    .unwrap();

    let cfg = Config::from_dotenv(&path).unwrap();
    assert_eq!(cfg.port, 8080);
    assert_eq!(cfg.db_pool_size, 4);

    // generated schema still validates the same map
    let vars = envro::load_dotenv(&path).unwrap();
    envro::validate(&vars, &Config::schema()).unwrap();

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
#[serial_test::serial]
fn from_env_reads_process() {
    std::env::set_var("APP_NAME", "envro");
    std::env::set_var("PORT", "9090");
    std::env::set_var("FEATURE_X", "1");
    std::env::set_var("DB_POOL_SIZE", "8");
    std::env::remove_var("OFFSET");

    let cfg = Config::from_env().unwrap();
    assert_eq!(cfg.port, 9090);
    assert_eq!(cfg.offset, None);
    assert_eq!(cfg.db_pool_size, 8);

    std::env::remove_var("APP_NAME");
    std::env::remove_var("PORT");
    std::env::remove_var("FEATURE_X");
    std::env::remove_var("DB_POOL_SIZE");
}
