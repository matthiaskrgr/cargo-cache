use std::process::Command;

pub(crate) struct VersionInfo {
    major: u8,
    minor: u8,
    patch: u16,
    commit_hash: String,
    commit_date: String,
}

impl VersionInfo {
    pub(crate) fn new() -> Self {
        // these are set by cargo
        let major = env!("CARGO_PKG_VERSION_MAJOR").parse::<u8>().unwrap();
        let minor = env!("CARGO_PKG_VERSION_MINOR").parse::<u8>().unwrap();
        let patch = env!("CARGO_PKG_VERSION_PATCH").parse::<u16>().unwrap();
        // for commit hash and date we have to dive a bit deeper.
        // code inspired by rls

        let commit_hash = String::from_utf8(
            Command::new("git")
                .args(&["rev-parse", "--short", "HEAD"])
                .output()
                .expect("'git rev-parse --short HEAD' failed")
                .stdout,
        ).unwrap()
        .trim()
        .to_string();

        let commit_date = String::from_utf8(
            Command::new("git")
                .args(&["log", "-1", "--date=short", "--pretty=format:%cd"])
                .output()
                .expect("git log -1 --date=short --pretty=format:%cd' failed")
                .stdout,
        ).unwrap()
        .trim()
        .to_string();

        VersionInfo {
            major,
            minor,
            patch,
            commit_hash,
            commit_date,
        }
    }
}

impl std::fmt::Display for Self {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}.{}.{} ({} {})",
            self.major,
            self.minor,
            self.patch,
            self.commit_hash,
            self.commit_date
        )?;
        Ok(())
    }
}
