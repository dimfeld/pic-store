use sea_orm_migration::{
    sea_orm::{ConnectionTrait, DatabaseTransaction, EntityTrait, Schema},
    DbErr,
};

pub async fn create_table_and_indexes<E: EntityTrait>(
    txn: &DatabaseTransaction,
    e: E,
) -> Result<(), DbErr> {
    let db = txn.get_database_backend();
    let schema = Schema::new(db);

    let create_query = schema.create_table_from_entity(e);
    txn.execute(db.build(&create_query)).await?;

    for idx in schema.create_index_from_entity(e) {
        txn.execute(db.build(&idx)).await?;
    }

    Ok(())
}
