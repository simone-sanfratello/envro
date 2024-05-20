use std::collections::HashMap;
use std::io;
use std::{env, fs, path::Path};

// TODO better / more standard error handling?
#[derive(Debug, thiserror::Error)]
pub enum EnvroError {
    #[error("FILE_ERROR unable to read env file {file:?}: {source:?}")]
    File {
        #[source]
        source: io::Error,
        file: String,
    },
    #[error("PARSE_ERROR line {line:?} is not valid")]
    Parse { line: String },
}

pub type EnvroVars = HashMap<String, String>;

/// load .env file into process.env var
///
/// # Examples
///
/// ```
/// use std::env;
/// use envro::*;
///
/// let env_file = env::current_dir().unwrap().join(".env-sample");
/// let env_vars = load_dotenv(&env_file).unwrap();
/// ```
pub fn load_dotenv(file_name: &Path) -> Result<EnvroVars, EnvroError> {
    let file_content = match fs::read_to_string(file_name) {
        Ok(c) => c,
        Err(err) => {
            return Err(EnvroError::File {
                source: err,
                file: String::from(file_name.to_str().unwrap_or("unknow file name")),
            })
        }
    };

    let vars = parse_env_file_content(file_content);

    Ok(vars)
}

#[derive(Debug, PartialEq, Eq, Hash)]
enum State {
    OpenValueQuote,
    CloseValueQuote,
    Variable,
    Value,

    Invalid,
}

#[macro_export]
macro_rules! spacing {
    ( $c:tt ) => {
        *$c == ' ' || *$c == '\n' || *$c == '\t' || *$c == '\r'
    };
}

fn parse_env_file_content(file_content: String) -> EnvroVars {
    let mut vars = EnvroVars::new();

    let chars: Vec<char> = file_content.chars().collect::<Vec<_>>();

    let len = chars.len();
    let mut index: usize = 0;
    let mut state = State::Variable;

    while index < len {
        let c: &char = &chars[index];
        loop {
            if spacing!(c) {
                break;
            }
            match state {
                State::Variable => {
                    // collect til =
                    // TODO allowed chars option
                }
                State::Value => {
                    //     quoted_value(&mut p, &chars, &len, false);
                    //     p.state = State::CloseValueQuote;
                    //     break;
                    // }
                }

                _ => {
                    state = State::Invalid;
                    break;
                }
            }
        }

        // println!("{:?} {}", p.state, chars[p.index]);
        index += 1;
    }

    // if state == invalid

    //     vars.insert(String::from(v[0]), String::from(v[1]));
    // }

    vars
}

/// load vars from env file and set them in env vars, overriding
///
/// # Examples
///
/// ```
/// use std::env;
/// use envro::*;
//
/// let env_file = env::current_dir().unwrap().join(".env-sample");
/// let env_vars = load_dotenv_in_env_vars(&env_file).unwrap();
/// ```
pub fn load_dotenv_in_env_vars(file_name: &Path) -> Result<(), EnvroError> {
    let vars = load_dotenv(file_name)?;

    for (key, value) in vars {
        env::set_var(key, value);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::{env, fs::File, io::Write};

    use super::*;

    #[test]
    fn should_load_a_simple_dotenv_file() {
        let file_name = env::temp_dir().join(".env-simple");
        let mut file = File::create(&file_name).unwrap();
        file.write_all(b"VAR=value").unwrap();
        env::remove_var("VAR");

        load_dotenv_in_env_vars(file_name.as_path()).unwrap();

        assert_eq!(env::var("VAR"), Ok("value".to_string()));
    }

    #[cfg(not(target_os = "windows"))]
    #[test]
    fn should_handle_error_on_non_existing_dotenv_file() {
        let r = load_dotenv(Path::new("none"));
        let err = r.unwrap_err();

        assert_eq!(
            err.to_string(),
            String::from(
                r#"FILE_ERROR unable to read env file "none": Os { code: 2, kind: NotFound, message: "No such file or directory" }"#
            )
        );
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn should_handle_error_on_non_existing_dotenv_file_on_win() {
        let r = load_dotenv(Path::new("none"));
        let err = r.unwrap_err();

        assert!(err.to_string().starts_with(
            r#"FILE_ERROR unable to read env file "none": Os { code: 2, kind: NotFound"#
        ));
    }

    #[cfg(not(target_os = "windows"))]
    #[test]
    fn should_handle_error_on_non_existing_dotenv_file_name_empty() {
        let r = load_dotenv(Path::new(""));
        let err = r.unwrap_err();

        assert_eq!(
            err.to_string(),
            String::from(
                r#"FILE_ERROR unable to read env file "": Os { code: 2, kind: NotFound, message: "No such file or directory" }"#
            )
        );
    }

    #[test]
    fn should_handle_error_on_invalid_dotenv_line() {
        let file_name = env::temp_dir().join(".env-invalid-line");
        let mut file = File::create(&file_name).unwrap();
        file.write_all(b"VAR value").unwrap();

        let r = load_dotenv(file_name.as_path());
        let err = r.unwrap_err();

        assert_eq!(
            err.to_string(),
            String::from(r#"PARSE_ERROR line "VAR value" is not valid"#)
        );
    }

    #[test]
    fn should_handle_error_on_invalid_dotenv_var() {
        let file_name = env::temp_dir().join(".env-invalid-var");
        let mut file = File::create(&file_name).unwrap();
        file.write_all(b"=value").unwrap();

        let r = load_dotenv(file_name.as_path());
        let err = r.unwrap_err();

        assert_eq!(
            err.to_string(),
            String::from(r#"PARSE_ERROR line "=value" is not valid"#)
        );
    }

    #[test]
    fn should_handle_error_on_invalid_dotenv_value() {
        let file_name = env::temp_dir().join(".env-invalid-value");
        let mut file = File::create(&file_name).unwrap();
        file.write_all(b"VAR=").unwrap();

        let r = load_dotenv(file_name.as_path());
        let err = r.unwrap_err();

        assert_eq!(
            err.to_string(),
            String::from(r#"PARSE_ERROR line "VAR=" is not valid"#)
        );
    }

    #[test]
    fn should_handle_empty_lines() {
        let file_name = env::temp_dir().join(".env-empty-lines");
        let mut file = File::create(&file_name).unwrap();
        file.write_all(b"\nVAR=1\nVAR1=asd").unwrap();
        env::remove_var("VAR");
        env::remove_var("VAR1");

        load_dotenv(file_name.as_path()).unwrap();

        assert_eq!(env::var("VAR"), Ok("1".to_string()));
        assert_eq!(env::var("VAR1"), Ok("asd".to_string()));
    }

    #[test]
    fn should_handle_comment_lines() {
        let file_name = env::temp_dir().join(".env-empty-lines");
        let mut file = File::create(&file_name).unwrap();
        file.write_all(b"\nVAR=1\n#VAR1=asd").unwrap();
        env::remove_var("VAR");
        env::remove_var("VAR1");

        load_dotenv(file_name.as_path()).unwrap();

        assert_eq!(env::var("VAR"), Ok("1".to_string()));
        assert_eq!(env::var("VAR1"), Err(env::VarError::NotPresent));
    }

    #[test]
    fn should_handle_quoted_values() {
        let file_name = env::temp_dir().join(".env-quoted");
        let mut file = File::create(&file_name).unwrap();
        file.write_all(b"\nVAR1=\"1\"\nVAR2=\"Lorem ipsum \"ciao!\" \"")
            .unwrap();
        env::remove_var("VAR1");
        env::remove_var("VAR2");

        load_dotenv(file_name.as_path()).unwrap();

        assert_eq!(env::var("VAR1"), Ok("1".to_string()));
        assert_eq!(env::var("VAR2"), Ok("Lorem ipsum \"ciao!\" ".to_string()));
    }
}
