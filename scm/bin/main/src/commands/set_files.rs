use super::{attach, expect_value, parse_value, validate_connect_args, CliError};

pub fn execute(args: &[String]) -> Result<(), CliError> {
    let mut port: Option<u16> = None;
    let mut package: Option<String> = None;
    let mut selector: Option<String> = None;
    let mut files: Option<String> = None;

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--port" => {
                i += 1;
                port = Some(parse_value(args, i, "--port")?);
            }
            "--package" => {
                i += 1;
                package = Some(expect_value(args, i, "--package")?);
            }
            "--selector" => {
                i += 1;
                selector = Some(expect_value(args, i, "--selector")?);
            }
            "--files" => {
                i += 1;
                files = Some(expect_value(args, i, "--files")?);
            }
            other => return Err(CliError::InvalidArgs(format!("unknown option: {}", other))),
        }
        i += 1;
    }
    validate_connect_args(port, &package)?;

    let selector = selector.ok_or_else(|| CliError::InvalidArgs("--selector is required".to_string()))?;
    let files = files.ok_or_else(|| CliError::InvalidArgs("--files is required".to_string()))?;
    let file_paths = parse_file_list(&files)?;

    let client = attach(port, package.as_deref())?;
    client.set_files(&selector, &file_paths).map_err(CliError::ExecutionFailed)?;

    println!("Set {} file(s) on {}", file_paths.len(), selector);
    Ok(())
}

/// Split `--files`'s comma-separated value into individual paths, trimming
/// whitespace and rejecting an empty list (e.g. `--files ""` or `--files ,`).
fn parse_file_list(raw: &str) -> Result<Vec<String>, CliError> {
    let paths: Vec<String> = raw
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    if paths.is_empty() {
        return Err(CliError::InvalidArgs("--files must contain at least one path".to_string()));
    }
    Ok(paths)
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_file_list_splits_on_comma() {
        assert_eq!(
            parse_file_list("a.png,b.pdf").unwrap(),
            vec!["a.png".to_string(), "b.pdf".to_string()]
        );
    }

    #[test]
    fn test_parse_file_list_trims_whitespace_around_entries() {
        assert_eq!(
            parse_file_list(" a.png , b.pdf ").unwrap(),
            vec!["a.png".to_string(), "b.pdf".to_string()]
        );
    }

    #[test]
    fn test_parse_file_list_accepts_a_single_path() {
        assert_eq!(parse_file_list("only.png").unwrap(), vec!["only.png".to_string()]);
    }

    #[test]
    fn test_parse_file_list_rejects_empty_string() {
        assert!(parse_file_list("").is_err());
    }

    #[test]
    fn test_parse_file_list_rejects_only_commas() {
        assert!(parse_file_list(",,").is_err());
    }
}
