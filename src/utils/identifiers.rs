/// Checks if the character is a valid GitArena identifier.
///
/// Valid GitArena identifiers are either alphanumeric (`a-z`, `0-9`), dash (`-`) or underscore (`_`).
/// This is used for all sort of name validating including user names and repository names.
///
/// # Example
///
/// ```
/// use crate::utils::identifiers::is_valid;
///
/// let valid_name = "mellowagain";
/// assert!(valid_name.chars().all(|c| is_valid(&c)));
///
/// let invalid_name = "invalid!name";
/// assert!(!invalid_name.chars().all(|c| is_valid(&c)));
/// ```
pub(crate) fn is_valid(c: &char) -> bool {
    c.is_ascii_alphanumeric() || c == &'-' || c == &'_'
}

/// Checks if the string is a reserved username.
///
/// This method checks the input string against the list of hardcoded, reserved usernames.
/// These exist to prevent route mismatches, as user profiles are located at `/<username>`.
///
/// For example: `login` is a reserved username as `/login` is the route used for logging in.
/// If somebody registered `login` as their username, their profile would not be visitable.
///
/// # Example
///
/// ```
/// use crate::utils::identifiers::is_reserved_username;
///
/// assert!(is_reserved_username("mellowagain")); // Valid
/// assert!(!is_reserved_username("login")); // Invalid
/// ```
pub(crate) fn is_reserved_username(input: &str) -> bool {
    const ILLEGAL_USERNAMES: [&str; 6] = [
        "admin",
        "api",
        "login",
        "logout",
        "register",
        "static"
    ];

    let lower_case = input.to_lowercase();
    ILLEGAL_USERNAMES.contains(&lower_case.as_str())
}

/// Checks if the string is a reserved repository name.
///
/// This method checks the input string against the list of hardcoded, reserved repository names.
/// These exist to prevent route mismatches, as repositories are located at `/<username>/<repo name>`.
///
/// For example: `repositories` is a reserved repo name as `/<username>/repositories` is the route used for viewing a list of an users repositories.
/// If somebody created a repository with the name `repositories`, their repository would not be visitable.
///
/// # Example
///
/// ```
/// use crate::utils::identifiers::is_reserved_repo_name;
///
/// assert!(is_reserved_repo_name("gitarena")); // Valid
/// assert!(!is_reserved_repo_name("repositories")); // Invalid
/// ```
pub(crate) fn is_reserved_repo_name(input: &str) -> bool {
    const ILLEGAL_REPO_NAMES: [&str; 1] = [
        "repositories"
    ];

    let lower_case = input.to_lowercase();
    ILLEGAL_REPO_NAMES.contains(&lower_case.as_str())
}

/// Checks if the string is a legal name for this operating system.
///
/// On Windows, this checks the input against the a of hardcoded, illegal file names.
/// On other operating systems, this function will always return `true`.
///
/// # Example
///
/// ```no_run
/// use crate::utils::identifiers::is_fs_legal;
///
/// // This will fail on Windows but pass on other operating systems
/// assert!(is_fs_legal("COM0"));
/// ```
pub(crate) fn is_fs_legal(input: &str) -> bool {
    // The actual implementation is in private functions to prevent having to write the doc twice
    internal_is_fs_legal(input)
}

#[cfg(windows)]
fn internal_is_fs_legal(input: &str) -> bool {
    const ILLEGAL_FILENAMES: [&str; 25] = [
        "con", "prn", "aux", "nul", "lst",
        "com0", "com1", "com2", "com3", "com4", "com5", "com6", "com7", "com8", "com9",
        "lpt0", "lpt1", "lpt2", "lpt3", "lpt4", "lpt5", "lpt6", "lpt7", "lpt8", "lpt9"
    ];

    // Strip the extension if one exists (as Windows ignores them as well)
    let lowercase = if let Some((file_name, _)) = input.rsplit_once('.') {
        file_name
    } else {
        input
    }.to_lowercase(); // These invalid file names are valid for both cases

    !ILLEGAL_FILENAMES.contains(&lowercase.as_str())
}

#[cfg(not(windows))]
#[inline]
fn internal_is_fs_legal(_: &str) -> bool {
    true
}
