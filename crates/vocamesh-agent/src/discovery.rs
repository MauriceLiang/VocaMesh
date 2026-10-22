use vocamesh_core::Provider;

/// Extract the first dotted numeric version token without retaining other CLI output.
pub fn parse_version(output: &str) -> Option<String> {
    let output = output.trim();
    if output.is_empty() {
        return None;
    }
    for token in output.split_whitespace() {
        let token = token.trim_matches(|character: char| {
            !character.is_ascii_alphanumeric() && !matches!(character, '.' | '+' | '-')
        });
        let token = token.strip_prefix('v').unwrap_or(token);
        let (without_build, build) = token.split_once('+').unwrap_or((token, ""));
        if !build.is_empty()
            && !build
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-')
        {
            continue;
        }
        let (numeric, prerelease) = without_build.split_once('-').unwrap_or((without_build, ""));
        let segments: Vec<&str> = numeric.split('.').collect();
        if segments.len() < 2
            || segments
                .iter()
                .any(|part| part.is_empty() || !part.chars().all(|c| c.is_ascii_digit()))
        {
            continue;
        }
        let suffix_is_valid = prerelease.is_empty()
            || prerelease
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-');
        if suffix_is_valid {
            return Some(token.to_owned());
        }
    }
    None
}

pub fn executable_name(provider: Provider) -> &'static str {
    provider.executable()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_only_numeric_version_tokens() {
        assert_eq!(
            parse_version("Claude Code 2.1.4 (Claude Code)"),
            Some("2.1.4".into())
        );
        assert_eq!(
            parse_version("codex-cli 0.12.1-alpha.2\n"),
            Some("0.12.1-alpha.2".into())
        );
        assert_eq!(
            parse_version("opencode v1.2.3+build.5"),
            Some("1.2.3+build.5".into())
        );
        assert_eq!(parse_version("no version here"), None);
        assert_eq!(executable_name(Provider::Opencode), "opencode");
    }
}
