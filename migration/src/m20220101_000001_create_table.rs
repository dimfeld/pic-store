use crate::create_table_and_index::create_table_and_indexes;
use pic_store_db::*;
use sea_orm_migration::{
    prelude::*,
    sea_orm::{ConnectionTrait, TransactionTrait},
};

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20220101_000001_create_table"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        let txn = db.begin().await?;

        create_table_and_indexes(&txn, team::Entity).await?;
        create_table_and_indexes(&txn, user::Entity).await?;
        create_table_and_indexes(&txn, project::Entity).await?;
        create_table_and_indexes(&txn, storage_location::Entity).await?;
        create_table_and_indexes(&txn, conversion_profile::Entity).await?;
        create_table_and_indexes(&txn, conversion_profile_item::Entity).await?;
        create_table_and_indexes(&txn, upload_profile::Entity).await?;
        create_table_and_indexes(&txn, base_image::Entity).await?;
        create_table_and_indexes(&txn, output_image::Entity).await?;

        txn.commit().await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let conn = manager.get_connection();
        let db = conn.get_database_backend();
        let txn = conn.begin().await?;

        txn.execute(
            db.build(
                &sea_query::Table::drop()
                    .table(output_image::Entity)
                    .to_owned(),
            ),
        )
        .await?;

        txn.execute(
            db.build(
                &sea_query::Table::drop()
                    .table(base_image::Entity)
                    .to_owned(),
            ),
        )
        .await?;

        txn.execute(
            db.build(
                &sea_query::Table::drop()
                    .table(upload_profile::Entity)
                    .to_owned(),
            ),
        )
        .await?;

        txn.execute(
            db.build(
                &sea_query::Table::drop()
                    .table(conversion_profile_item::Entity)
                    .to_owned(),
            ),
        )
        .await?;

        txn.execute(
            db.build(
                &sea_query::Table::drop()
                    .table(conversion_profile::Entity)
                    .to_owned(),
            ),
        )
        .await?;

        txn.execute(
            db.build(
                &sea_query::Table::drop()
                    .table(storage_location::Entity)
                    .to_owned(),
            ),
        )
        .await?;

        txn.execute(db.build(&sea_query::Table::drop().table(project::Entity).to_owned()))
            .await?;

        txn.execute(db.build(&sea_query::Table::drop().table(user::Entity).to_owned()))
            .await?;

        txn.execute(db.build(&sea_query::Table::drop().table(team::Entity).to_owned()))
            .await?;

        txn.commit().await?;
        Ok(())
    }
}
