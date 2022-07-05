mod error;
mod eval;
mod extract_token;
mod fact;
mod parse_biscuit;
mod request;

use biscuit_auth::{Biscuit, KeyPair};

pub use error::*;
pub use eval::*;
pub use fact::*;
pub use parse_biscuit::*;
pub use request::*;

pub struct BiscuitBuilder {
    keypair: KeyPair,
}

impl BiscuitBuilder {
    pub fn new(keypair: KeyPair) -> BiscuitBuilder {
        BiscuitBuilder { keypair }
    }

    pub fn generate_token_for_user(
        &self,
        ids: &UserAndTeamIds,
    ) -> Result<biscuit_auth::builder::BiscuitBuilder, biscuit_auth::error::Token> {
        let mut builder = Biscuit::builder(&self.keypair);

        builder.add_authority_fact(Fact::User.with_value(ids.user_id))?;
        builder.add_authority_fact(Fact::UserTeam.with_value(ids.team_id))?;

        Ok(builder)
    }
}

#[cfg(test)]
mod tests {
    use uuid::Uuid;

    use crate::UserAndTeamIds;

    use super::BiscuitBuilder;

    #[test]
    fn build_token() {
        let builder = BiscuitBuilder::new(biscuit_auth::KeyPair::new());
        let user_id =
            Uuid::parse_str("79B9E1A8-EBBA-4353-B536-FEFD5C99BCC5").expect("creating uuid");
        let team_id =
            Uuid::parse_str("BEC86DB7-95C4-4F7A-AC35-5EBC8C4AE109").expect("creating uuid");

        let token = builder
            .generate_token_for_user(&UserAndTeamIds { user_id, team_id })
            .expect("Generating token")
            .build()
            .expect("Generating token");
        assert_eq!(
            token.print_block_source(0).unwrap(),
            "user(\"79b9e1a8-ebba-4353-b536-fefd5c99bcc5\");\nteam(\"bec86db7-95c4-4f7a-ac35-5ebc8c4ae109\");\n"
        );
        assert_eq!(token.block_count(), 1, "Just one block");
    }
}
