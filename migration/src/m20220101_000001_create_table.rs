use pic_store_db::*;
use sea_orm_migration::{prelude::*, sea_orm::Schema};

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20220101_000001_create_table"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_database_backend();
        let schema = Schema::new(db);

        manager
            .create_table(schema.create_table_from_entity(team::Entity))
            .await?;
        for idx in schema.create_index_from_entity(team::Entity) {
            manager.create_index(idx).await?;
        }

        manager
            .create_table(schema.create_table_from_entity(user::Entity))
            .await?;
        for idx in schema.create_index_from_entity(user::Entity) {
            manager.create_index(idx).await?;
        }

        manager
            .create_table(schema.create_table_from_entity(base_image::Entity))
            .await?;
        for idx in schema.create_index_from_entity(base_image::Entity) {
            manager.create_index(idx).await?;
        }

        manager
            .create_table(schema.create_table_from_entity(output_image::Entity))
            .await?;
        for idx in schema.create_index_from_entity(output_image::Entity) {
            manager.create_index(idx).await?;
        }

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(
                sea_query::Table::drop()
                    .table(output_image::Entity)
                    .to_owned(),
            )
            .await?;
        manager
            .drop_table(
                sea_query::Table::drop()
                    .table(base_image::Entity)
                    .to_owned(),
            )
            .await?;
        manager
            .drop_table(sea_query::Table::drop().table(user::Entity).to_owned())
            .await?;
        manager
            .drop_table(sea_query::Table::drop().table(team::Entity).to_owned())
            .await?;
        Ok(())
    }
}
