use std::{fmt::Display, ops::Deref};

#[derive(Debug, Copy, Clone)]
pub enum Fact {
    Operation,
    User,
    UserTeam,
    Project,
    Resource,
    ResourceTeam,
    ResourceType,
    Deleted,
}

impl Fact {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Fact::Operation => "operation",
            Fact::User => "user",
            Fact::UserTeam => "team",
            Fact::Project => "project",
            Fact::ResourceTeam => "resource_team",
            Fact::Resource => "resource",
            Fact::ResourceType => "resource_type",
            Fact::Deleted => "deleted",
        }
    }

    pub fn with_value(&self, value: impl ToString) -> biscuit_auth::builder::Fact {
        biscuit_auth::builder::Fact::new(
            self.to_string(),
            vec![biscuit_auth::builder::Term::Str(value.to_string())],
        )
    }

    pub fn check_if(&self, value: &str) -> String {
        format!(r##"check if {}("{}")"##, self.as_str(), value)
    }
}

impl Display for Fact {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl Deref for Fact {
    type Target = str;

    fn deref(&self) -> &'static Self::Target {
        self.as_str()
    }
}

#[cfg(test)]
mod tests {
    use super::Fact;

    #[test]
    fn as_str() {
        assert_eq!(Fact::Operation.as_str(), "operation")
    }

    #[test]
    fn with_value() {
        assert_eq!(
            Fact::Operation.with_value("write").to_string(),
            r##"operation("write")"##
        )
    }

    #[test]
    fn check_if() {
        assert_eq!(
            Fact::Operation.check_if("write"),
            r##"check if operation("write")"##
        )
    }
}
