use clap::{Args, Subcommand};

use pic_store_db::object_id;

#[derive(Debug, Args)]
pub struct AdminArgs {
    #[clap(subcommand)]
    commands: Commands,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Create an object ID
    ///
    /// This is useful for generating a package of initial data, such as the first team and user,
    /// or for other testing.
    MakeId(MakeId),
}

#[derive(Debug, Args)]
pub struct MakeId {
    #[clap(subcommand)]
    command: IdType,
}

#[derive(Debug, Subcommand)]
enum IdType {
    Team,
    Role,
    User,
    Project,
    ConversionProfile,
    StorageLocation,
    UploadProfile,
    BaseImage,
    OutputImage,
}

pub fn admin_commands(cmd: AdminArgs) {
    match cmd.commands {
        Commands::MakeId(MakeId { command }) => make_id(command),
    }
}

fn make_id(id: IdType) {
    let id = match id {
        IdType::Team => object_id::TeamId::new().to_string(),
        IdType::User => object_id::UserId::new().to_string(),
        IdType::Role => object_id::RoleId::new().to_string(),
        IdType::Project => object_id::ProjectId::new().to_string(),
        IdType::ConversionProfile => object_id::ConversionProfileId::new().to_string(),
        IdType::StorageLocation => object_id::StorageLocationId::new().to_string(),
        IdType::UploadProfile => object_id::UploadProfileId::new().to_string(),
        IdType::BaseImage => object_id::BaseImageId::new().to_string(),
        IdType::OutputImage => object_id::OutputImageId::new().to_string(),
    };

    println!("{id}");
}
