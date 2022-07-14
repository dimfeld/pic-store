#[macro_export]
macro_rules! diesel_jsonb {
    ($type: ty) => {
        impl ::diesel::deserialize::FromSql<::diesel::sql_types::Jsonb, ::diesel::pg::Pg>
            for $type
        {
            fn from_sql(
                value: diesel::backend::RawValue<'_, diesel::pg::Pg>,
            ) -> ::diesel::deserialize::Result<Self> {
                let bytes = value.as_bytes();
                if bytes[0] != 1 {
                    return Err("Unsupported JSONB encoding version".into());
                }

                ::serde_json::from_slice(&bytes[1..])
                    .map_err(|e| format!("Invalid JSON: {}", e).into())
                // let value = <serde_json::Value as ::diesel::deserialize::FromSql<
                //     ::diesel::sql_types::Jsonb,
                //     ::diesel::pg::Pg,
                // >>::from_sql(value)?;
                // Ok(serde_json::from_value(value)?)
            }
        }

        impl ::diesel::serialize::ToSql<::diesel::sql_types::Jsonb, ::diesel::pg::Pg> for $type {
            fn to_sql(
                &self,
                out: &mut ::diesel::serialize::Output<diesel::pg::Pg>,
            ) -> ::diesel::serialize::Result {
                use std::io::Write;

                out.write_all(&[1])?;
                serde_json::to_writer(out, self)
                    .map(|_| diesel::serialize::IsNull::No)
                    .map_err(Into::into)
                // let value = serde_json::to_value(self)?;
                // <serde_json::Value as diesel::serialize::ToSql<
                //     diesel::sql_types::Jsonb,
                //     diesel::pg::Pg,
                // >>::to_sql(&value, &mut out.reborrow())
            }
        }
    };
}
