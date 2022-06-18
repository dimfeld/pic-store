mod extract_token;
mod fact;
mod parse_biscuit;
mod request;

use biscuit_auth::{Biscuit, KeyPair};

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
        team_id: &str,
        user_id: &str,
    ) -> Result<biscuit_auth::builder::BiscuitBuilder, biscuit_auth::error::Token> {
        let mut builder = Biscuit::builder(&self.keypair);

        builder.add_authority_fact(Fact::User.with_value(user_id))?;
        builder.add_authority_fact(Fact::Team.with_value(team_id))?;

        Ok(builder)
    }
}
