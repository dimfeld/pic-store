use std::ops::Deref;

#[derive(Debug, Copy, Clone)]
pub enum Fact {
    Operation,
    Team,
    User,
    Project,
    UploadProfile,
    Image,
}

impl Fact {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Fact::Operation => "operation",
            Fact::Team => "team",
            Fact::User => "user",
            Fact::Project => "project",
            Fact::UploadProfile => "profile",
            Fact::Image => "image",
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

impl ToString for Fact {
    fn to_string(&self) -> String {
        self.as_str().to_string()
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
