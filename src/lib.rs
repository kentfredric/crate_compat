#![cfg(test)]
use semver::Version;
use semver::VersionReq;
use std::fmt::Display;
use std::fmt::Error;
use url::Url;

enum Target {
    Crate(&'static str, VersionReq),
    Rust(VersionReq),
}
enum RefType {
    Bug(Url),
    PullRequest(Url),
    Commit(Url),
}
struct IncompatRecord {
    target: Target,
    conflicts: Target,
    reason: Option<&'static str>,
    references: Option<Vec<RefType>>,
}
impl IncompatRecord {
    fn affects_crate(&self, affected_crate: &str) -> bool {
        if let Target::Crate(xcrate, _) = self.target {
            xcrate == affected_crate
        } else {
            false
        }
    }
    fn affects(&self, affected_crate: &str, req: Version) -> bool {
        if let Target::Crate(xcrate, crate_req) = &self.target {
            xcrate == &affected_crate && crate_req.matches(&req)
        } else {
            false
        }
    }
    fn has_conflicts(&self, conflicting_crate: &str) -> bool {
        if let Target::Crate(xcrate, _) = self.conflicts {
            xcrate == conflicting_crate
        } else {
            false
        }
    }

    fn conflicts(&self, conflicting_crate: &str, req: Version) -> bool {
        if let Target::Crate(xcrate, crate_req) = &self.conflicts {
            xcrate == &conflicting_crate && crate_req.matches(&req)
        } else {
            false
        }
    }

    fn has_rust_conflicts(&self) -> bool {
        if let Target::Rust(_) = self.conflicts {
            true
        } else {
            false
        }
    }

    fn rust_conflicts(&self, req: Version) -> bool {
        if let Target::Rust(rust_req) = &self.conflicts {
            rust_req.matches(&req)
        } else {
            false
        }
    }
}

impl Display for IncompatRecord {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> Result<(), Error> {
        write!(formatter, "{} with {}\n", &self.target, &self.conflicts)?;
        if let Some(reason) = self.reason {
            write!(formatter, "- {}\n", reason)?;
        }
        if let Some(references) = &self.references {
            if references.len() > 0 {
                write!(formatter, "References:\n")?;
                for reference in references {
                    write!(formatter, "- {}\n", reference)?;
                }
            }
        }
        Ok(())
    }
}

impl Display for Target {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> Result<(), Error> {
        match &self {
            Target::Rust(rust_req) => write!(formatter, "rust({})", rust_req),
            Target::Crate(name, req) => write!(formatter, "crate({} {})", name, req),
        }
    }
}

impl Display for RefType {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> Result<(), Error> {
        match &self {
            RefType::Bug(url) => write!(formatter, "Bug: {}", url),
            RefType::PullRequest(url) => write!(formatter, "Pull: {}", url),
            RefType::Commit(url) => write!(formatter, "Commit: {}", url),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::IncompatRecord;
    use super::RefType::*;
    use super::Target::*;
    use lazy_static::lazy_static;
    use semver::VersionReq;
    use url::Url;

    lazy_static! {
        static ref FAILURE_DERIVE: IncompatRecord = IncompatRecord {
            target: Crate("failure_derive", VersionReq::parse("< 1.0.7").unwrap()),
            conflicts: Crate("quote", VersionReq::parse(">= 1.0.3").unwrap()),
            reason: Some("Broken by rename of quote::_rt to quote::_private in 1.0.3"),
            references: Some(vec![
                Bug(Url::parse("https://github.com/withoutboats/failure_derive/issues/13").unwrap()),
                Bug(Url::parse("https://github.com/rust-lang-nursery/failure/issues/342").unwrap()),
                PullRequest(Url::parse("https://github.com/rust-lang-nursery/failure/pull/343").unwrap()),
                PullRequest(Url::parse("https://github.com/rust-lang-nursery/failure/pull/345").unwrap()),
                Commit(Url::parse("https://github.com/dtolnay/quote/commit/41543890aa76f4f8046fffac536b9445275aab26").unwrap()),
            ])
        };
        static ref FAILURE_BADRUST: IncompatRecord = IncompatRecord {
            target: Crate("failure_derive", VersionReq::parse("< 1.0.7").unwrap()),
            conflicts: Rust(VersionReq::parse("< 1.31").unwrap()),
            reason: Some("Documented minimum supported rust"),
            references: Some(vec![
                Commit(Url::parse("https://github.com/rust-lang-nursery/failure/commit/996f919f1e1741b08673b15f893221694097cc9f").unwrap())
            ])
        };
    }

    mod affects_crate {
        #[test]
        fn test_match() {
            assert!(super::FAILURE_DERIVE.affects_crate("failure_derive"));
            assert!(super::FAILURE_BADRUST.affects_crate("failure_derive"))
        }
        #[test]
        fn test_missmatch() {
            assert!(!super::FAILURE_DERIVE.affects_crate("failure_deriv"));
            assert!(!super::FAILURE_BADRUST.affects_crate("failure_deriv"))
        }
    }

    mod affects {
        use semver::Version;
        #[test]
        fn test_match() {
            assert!(
                super::FAILURE_DERIVE.affects("failure_derive", Version::parse("1.0.3").unwrap())
            );
            assert!(
                super::FAILURE_BADRUST.affects("failure_derive", Version::parse("1.0.3").unwrap())
            );
        }

        #[test]
        fn test_missmatch() {
            assert!(
                !super::FAILURE_DERIVE.affects("failure_derive", Version::parse("1.0.7").unwrap())
            );
            assert!(
                !super::FAILURE_BADRUST.affects("failure_derive", Version::parse("1.0.7").unwrap())
            )
        }
    }

    mod has_conflict {

        #[test]
        fn test_match() {
            assert!(super::FAILURE_DERIVE.has_conflicts("quote"))
        }
        #[test]
        fn test_missmatch() {
            assert!(!super::FAILURE_DERIVE.has_conflicts("quot"));
            assert!(!super::FAILURE_BADRUST.has_conflicts("quot"));
            assert!(!super::FAILURE_BADRUST.has_conflicts("quote"))
        }
    }

    mod conflicts {
        use semver::Version;
        #[test]
        fn test_match() {
            assert!(super::FAILURE_DERIVE.conflicts("quote", Version::parse("1.0.3").unwrap()))
        }

        #[test]
        fn test_missmatch() {
            assert!(!super::FAILURE_DERIVE.conflicts("quote", Version::parse("1.0.2").unwrap()));
            assert!(!super::FAILURE_BADRUST.conflicts("quote", Version::parse("1.0.2").unwrap()));
            assert!(!super::FAILURE_BADRUST.conflicts("quote", Version::parse("1.0.3").unwrap()))
        }
    }
    mod has_rust_conflicts {
        #[test]
        fn test_match() {
            assert!(super::FAILURE_BADRUST.has_rust_conflicts())
        }

        #[test]
        fn test_missmatch() {
            assert!(!super::FAILURE_DERIVE.has_rust_conflicts())
        }
    }

    mod rust_conflicts {
        use semver::Version;
        #[test]
        fn test_match() {
            assert!(super::FAILURE_BADRUST.rust_conflicts(Version::parse("1.30.0").unwrap()))
        }
    }

    mod display {
        #[test]
        fn test_display() {
            println!("{}", *super::FAILURE_DERIVE);
            println!("{}", *super::FAILURE_BADRUST);
        }
    }
}
