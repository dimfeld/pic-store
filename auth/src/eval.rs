use std::{
    fmt::{Debug, Display},
    ops::{Deref, DerefMut},
};

use axum::http::{Method, Request};
use biscuit_auth::{error, Authorizer, Biscuit};
use uuid::Uuid;

use crate::Error;

const ROOT_RULES: &str = r##"

"##;

#[derive(Clone)]
pub struct RootAuthEvaulator {
    root_authorizer: Authorizer<'static>,
}

impl RootAuthEvaulator {
    pub fn new() -> Self {
        let mut root_authorizer = Authorizer::new().expect("Creating root authorizer");

        // Uncomment this once we actually have some root rules
        // root_authorizer
        //     .add_code(ROOT_RULES)
        //     .expect("Adding root rules to authorizer");

        RootAuthEvaulator { root_authorizer }
    }

    pub fn with_biscuit<'a>(&self, token: &'a Biscuit) -> Result<AuthEvaluator<'a>, Error> {
        let authorizer = AuthEvaluator::new(self.root_authorizer.clone()).with_biscuit(token)?;
        Ok(authorizer)
    }

    /// Get an authorizer without associating it with a token.
    /// This can be useful when you want to do some other things with the authorizer first,
    /// and associating it with the lifetime of the token isn't convenient until later.
    pub fn get_authorizer(&self) -> AuthEvaluator<'static> {
        AuthEvaluator::new(self.root_authorizer.clone())
    }
}

impl Default for RootAuthEvaulator {
    fn default() -> Self {
        Self::new()
    }
}

impl Debug for RootAuthEvaulator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RootAuthEvaulator")
            .field("root_authorizer", &self.root_authorizer.print_world())
            .finish()
    }
}

/** A structure to help with evaluating authorization information, created by
 * RootAuthEvaulator */
#[derive(Clone)]
pub struct AuthEvaluator<'a> {
    pub authorizer: Authorizer<'a>,
}

impl<'a> AuthEvaluator<'a> {
    fn new(authorizer: Authorizer<'a>) -> Self {
        AuthEvaluator { authorizer }
    }

    // We need to restrict the lifetime of the Authorizer here, so consume it and return
    // it again.
    pub fn with_biscuit(mut self, token: &'a Biscuit) -> Result<AuthEvaluator<'a>, Error> {
        self.authorizer.add_token(token)?;
        Ok(self)
    }

    pub fn set_project(&mut self, project: impl ToString) -> Result<(), Error> {
        self.authorizer
            .add_fact(crate::Fact::Project.with_value(project))?;
        Ok(())
    }

    pub fn set_resource_type(&mut self, resource_type: impl ToString) -> Result<(), Error> {
        self.authorizer
            .add_fact(crate::Fact::ResourceType.with_value(resource_type))?;
        Ok(())
    }

    pub fn set_resource(&mut self, resource_id: impl ToString) -> Result<(), Error> {
        self.authorizer
            .add_fact(crate::Fact::Resource.with_value(resource_id))?;
        Ok(())
    }

    pub fn set_resource_team(&mut self, resource_team: impl ToString) -> Result<(), Error> {
        self.authorizer
            .add_fact(crate::Fact::ResourceTeam.with_value(resource_team))?;
        Ok(())
    }

    pub fn set_deleted(&mut self, deleted: bool) -> Result<(), Error> {
        let d_str = deleted.then_some("true").unwrap_or("false");
        self.authorizer
            .add_fact(crate::Fact::Deleted.with_value(d_str))?;
        Ok(())
    }

    pub fn set_operation_from_method(&mut self, method: &Method) -> Result<(), Error> {
        let operation = match *method {
            Method::GET => "read",
            Method::HEAD => "read",
            Method::POST => "create",
            Method::PUT => "write",
            Method::PATCH => "write",
            Method::DELETE => "delete",
            _ => "",
        };

        if !operation.is_empty() {
            self.authorizer
                .add_fact(crate::Fact::Operation.with_value(operation))?;
        }

        Ok(())
    }

    /// Get the user ID and team ID from the biscuit.
    pub fn get_user_and_team(&mut self) -> Result<UserAndTeamIds, Error> {
        let (team_id, user_id): (String, String) = self
            .authorizer
            .query(r##"data($team, $user) <- team($team), user($user)"##)?
            .pop()
            .ok_or(Error::MissingCredentials)?;

        let team_id = Uuid::parse_str(team_id.as_str()).map_err(|_| Error::IdParseError("team"))?;
        let user_id = Uuid::parse_str(user_id.as_str()).map_err(|_| Error::IdParseError("user"))?;

        Ok(UserAndTeamIds { team_id, user_id })
    }

    pub fn get_simple_fact(&mut self, fact: impl Display) -> Result<Option<String>, Error> {
        let query = format!(r##"data($value) <- {fact}($value)"##);
        let mut facts: Vec<(String,)> = self.authorizer.query(query.as_str())?;

        Ok(facts.pop().map(|s| s.0))
    }

    /// Get a singleton fact from the token, including any facts added after the initial
    /// generation.
    pub fn get_simple_fact_all(&mut self, fact: impl Display) -> Result<Option<String>, Error> {
        let query = format!(r##"data($value) <- {fact}($value)"##);
        let mut facts: Vec<(String,)> = self.authorizer.query_all(query.as_str())?;

        Ok(facts.pop().map(|s| s.0))
    }
}

impl<'a> Deref for AuthEvaluator<'a> {
    type Target = Authorizer<'a>;

    fn deref(&self) -> &Self::Target {
        &self.authorizer
    }
}

impl<'a> DerefMut for AuthEvaluator<'a> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.authorizer
    }
}

#[derive(Debug, Eq, PartialEq, Clone)]
pub struct UserAndTeamIds {
    pub user_id: Uuid,
    pub team_id: Uuid,
}

#[cfg(test)]
mod tests {
    use biscuit_auth::Biscuit;
    use uuid::Uuid;

    use crate::{BiscuitBuilder, Fact, RootAuthEvaulator, UserAndTeamIds};

    fn setup_with_builder() -> (RootAuthEvaulator, BiscuitBuilder, UserAndTeamIds) {
        let user_id =
            Uuid::parse_str("79B9E1A8-EBBA-4353-B536-FEFD5C99BCC5").expect("creating uuid");
        let team_id =
            Uuid::parse_str("BEC86DB7-95C4-4F7A-AC35-5EBC8C4AE109").expect("creating uuid");
        let builder = BiscuitBuilder::new(biscuit_auth::KeyPair::new());
        let ids = UserAndTeamIds { user_id, team_id };

        (RootAuthEvaulator::new(), builder, ids)
    }

    fn setup() -> (RootAuthEvaulator, Biscuit, UserAndTeamIds) {
        let (root_auth, builder, ids) = setup_with_builder();
        let token = builder
            .generate_token_for_user(&ids)
            .unwrap()
            .build()
            .unwrap();

        (root_auth, token, ids)
    }

    #[test]
    fn get_user_and_team() {
        let (root_auth, token, ids) = setup();
        let mut auth = root_auth.with_biscuit(&token).expect("Creating authorizer");

        let actual_ids = auth
            .get_user_and_team()
            .expect("Getting user and team from token");
        assert_eq!(actual_ids, ids);
    }

    #[test]
    fn set_and_retrieve_facts() {
        let (root_auth, token, _ids) = setup();
        let mut auth = root_auth.with_biscuit(&token).expect("Creating authorizer");

        auth.set_project("a-project").expect("Setting project");
        auth.set_resource("a-resource").expect("Setting resource");

        let project = auth
            .get_simple_fact_all(Fact::Project)
            .expect("Fetching project fact with get_simple_fact_all")
            .expect("Should find project fact with get_simple_fact_all");
        assert_eq!(project, "a-project");

        let resource = auth
            .get_simple_fact_all(Fact::Resource)
            .expect("Fetching resource fact with get_simple_fact_all")
            .expect("Should find resource fact with get_simple_fact_all");
        assert_eq!(resource, "a-resource");

        let project = auth
            .get_simple_fact(Fact::Project)
            .expect("Fetching project fact with get_simple_fact")
            .expect("Should find project fact with get_simple_fact");
        assert_eq!(project, "a-project");

        let resource = auth
            .get_simple_fact(Fact::Resource)
            .expect("Fetching resource fact with get_simple_fact")
            .expect("Should find resource fact with get_simple_fact");
        assert_eq!(resource, "a-resource");
    }

    #[test]
    fn get_simple_fact() {
        let (root_auth, token, ids) = setup();

        let mut next_block = token.create_block();
        next_block.add_fact(r##"new_fact("abc")"##).unwrap();
        let token = token.append(next_block).unwrap();

        println!("{:?}", token.print_block_source(0));
        println!("{:?}", token.print_block_source(1));

        let mut auth = root_auth.with_biscuit(&token).expect("Creating authorizer");

        let new_fact = auth
            .get_simple_fact("new_fact")
            .expect("Fetching resource fact with get_simple_fact");
        println!("new_fact: {new_fact:?}");
        assert!(
            new_fact.is_none(),
            "get_simple_fact should not find facts in added blocks",
        );

        // This seems to be broken right now.
        // let new_fact = auth
        //     .get_simple_fact_all("new_fact")
        //     .expect("Fetching new_fact with get_simple_fact_all");

        // println!("new_fact: {new_fact:?}");
        // let new_fact = new_fact.expect("Should find new_fact with get_simple_fact_all");
        // assert_eq!(new_fact.0, "abc");
    }
}
