use std::{ops::Deref, str::FromStr};

use base64::{display::Base64Display, engine::GeneralPurpose, Engine};
use diesel::{deserialize::FromSql, serialize::ToSql};
use thiserror::Error;
use uuid::Uuid;

use crate::new_uuid;

#[derive(Debug, Error)]
pub enum ObjectIdError {
    #[error("Invalid ID prefix, expected {0}")]
    InvalidPrefix(&'static str),

    #[error("Failed to decode object ID")]
    DecodeFailure,
}

/// A type that is internally stored as a UUID but externally as a
/// more accessible string with a prefix indicating its type.
#[derive(Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, AsExpression, FromSqlRow)]
#[diesel(sql_type = diesel::sql_types::Uuid)]
pub struct ObjectId<const PREFIX: usize>(pub Uuid);

pub type TeamId = ObjectId<0>;
pub type RoleId = ObjectId<1>;
pub type UserId = ObjectId<2>;
pub type ProjectId = ObjectId<3>;
pub type ConversionProfileId = ObjectId<4>;
pub type StorageLocationId = ObjectId<6>;
pub type UploadProfileId = ObjectId<7>;
pub type BaseImageId = ObjectId<8>;
pub type OutputImageId = ObjectId<9>;

impl<const PREFIX: usize> ObjectId<PREFIX> {
    /// Once const generics supports strings, this can go away, but for now we
    /// do it this way.
    #[inline(always)]
    fn prefix() -> &'static str {
        match PREFIX {
            0 => "tem",
            1 => "rol",
            2 => "usr",
            3 => "prj",
            4 => "cpr",
            5 => "cpi",
            6 => "slc",
            7 => "upl",
            8 => "bim",
            9 => "oim",
            _ => "",
        }
    }

    pub fn new() -> Self {
        Self(new_uuid())
    }

    pub fn from_uuid(u: Uuid) -> Self {
        Self(u)
    }

    pub fn into_inner(self) -> Uuid {
        self.0
    }

    pub fn as_uuid(&self) -> &Uuid {
        &self.0
    }

    pub fn nil() -> Self {
        Self(Uuid::nil())
    }

    pub fn display_without_prefix(&self) -> Base64Display<GeneralPurpose> {
        base64::display::Base64Display::new(
            self.0.as_bytes(),
            &base64::engine::general_purpose::URL_SAFE_NO_PAD,
        )
    }
}

impl<const PREFIX: usize> Default for ObjectId<PREFIX> {
    fn default() -> Self {
        Self::new()
    }
}

impl<const PREFIX: usize> PartialEq<Uuid> for ObjectId<PREFIX> {
    fn eq(&self, other: &Uuid) -> bool {
        &self.0 == other
    }
}

impl<const PREFIX: usize> Deref for ObjectId<PREFIX> {
    type Target = Uuid;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<const PREFIX: usize> From<Uuid> for ObjectId<PREFIX> {
    fn from(u: Uuid) -> Self {
        Self(u)
    }
}

impl<const PREFIX: usize> From<ObjectId<PREFIX>> for Uuid {
    fn from(data: ObjectId<PREFIX>) -> Self {
        data.0
    }
}

impl<const PREFIX: usize> std::fmt::Debug for ObjectId<PREFIX> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("ObjectId")
            .field(&self.to_string())
            .field(&self.0)
            .finish()
    }
}

impl<const PREFIX: usize> std::fmt::Display for ObjectId<PREFIX> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(Self::prefix())?;
        self.display_without_prefix().fmt(f)
    }
}

pub fn decode_suffix(s: &str) -> Result<Uuid, ObjectIdError> {
    let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(s)
        .map_err(|_| ObjectIdError::DecodeFailure)?;
    Uuid::from_slice(&bytes).map_err(|_| ObjectIdError::DecodeFailure)
}

impl<const PREFIX: usize> FromStr for ObjectId<PREFIX> {
    type Err = ObjectIdError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let expected_prefix = Self::prefix();
        if !s.starts_with(expected_prefix) {
            return Err(ObjectIdError::InvalidPrefix(expected_prefix));
        }

        decode_suffix(&s[expected_prefix.len()..]).map(Self)
    }
}

/// Serialize into string form with the prefix
impl<const PREFIX: usize> serde::Serialize for ObjectId<PREFIX> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let s = self.to_string();
        serializer.serialize_str(&s)
    }
}

struct ObjectIdVisitor<const PREFIX: usize>;

impl<'de, const PREFIX: usize> serde::de::Visitor<'de> for ObjectIdVisitor<PREFIX> {
    type Value = ObjectId<PREFIX>;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("an object ID starting with ")?;
        formatter.write_str(Self::Value::prefix())
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        match Self::Value::from_str(v) {
            Ok(id) => Ok(id),
            Err(e) => {
                // See if it's in UUID format instead of the encoded format. This mostly happens when
                // deserializing from a JSON object generated in Postgres with jsonb_build_object.
                Uuid::from_str(v)
                    .map(ObjectId::<PREFIX>::from_uuid)
                    // Return the more descriptive original error instead of the UUID parsing error
                    .map_err(|_| e)
            }
        }
        .map_err(|_| E::invalid_value(serde::de::Unexpected::Str(v), &self))
    }
}

/// Deserialize from string form with the prefix.
impl<'de, const PREFIX: usize> serde::Deserialize<'de> for ObjectId<PREFIX> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_str(ObjectIdVisitor)
    }
}

// impl<const PREFIX: usize> schemars::JsonSchema for ObjectId<PREFIX> {
//     fn schema_name() -> String {
//         String::schema_name()
//     }

//     fn json_schema(gen: &mut schemars::gen::SchemaGenerator) -> schemars::schema::Schema {
//         String::json_schema(gen)
//     }
// }

impl<const PREFIX: usize> FromSql<diesel::sql_types::Uuid, diesel::pg::Pg> for ObjectId<PREFIX> {
    fn from_sql(
        bytes: diesel::backend::RawValue<'_, diesel::pg::Pg>,
    ) -> diesel::deserialize::Result<Self> {
        <Uuid as FromSql<diesel::sql_types::Uuid, diesel::pg::Pg>>::from_sql(bytes).map(Self)
    }
}
impl<const PREFIX: usize> ToSql<::diesel::sql_types::Uuid, ::diesel::pg::Pg> for ObjectId<PREFIX> {
    fn to_sql(
        &self,
        out: &mut ::diesel::serialize::Output<diesel::pg::Pg>,
    ) -> diesel::serialize::Result {
        <Uuid as ToSql<diesel::sql_types::Uuid, diesel::pg::Pg>>::to_sql(
            &self.0,
            &mut out.reborrow(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn to_from_str() {
        let id = TeamId::new();

        let s = id.to_string();
        let id2 = TeamId::from_str(&s).unwrap();
        assert_eq!(id, id2, "ID converts to string and back");
    }

    #[test]
    fn serde() {
        let id = TeamId::new();
        let json_str = serde_json::to_string(&id).unwrap();
        let id2: TeamId = serde_json::from_str(&json_str).unwrap();
        assert_eq!(id, id2, "Value serializes and deserializes to itself");
    }
}
