use clap::{Args, Subcommand};

use pic_store_db::object_id;

#[derive(Debug, Args)]
pub struct AdminArgs {
    #[clap(subcommand)]
    commands: Commands,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Bootstrap the application from a set of files.
    ///
    /// Until there is a real admin interface this is the easiest way to create the initial team, user, project, etc.
    #[cfg(feature = "bootstrap")]
    Bootstrap(super::bootstrap::BootstrapArgs),
    /// Create an object ID
    ///
    /// This is useful for generating a package of initial data, such as the first team and user,
    /// or for other testing.
    MakeId(MakeId),
    /// Hash a password
    HashPassword(HashPassword),
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

#[derive(Debug, Args)]
pub struct HashPassword {
    /// The password to hash
    password: String,
}

pub fn admin_commands(cmd: AdminArgs) -> Result<(), anyhow::Error> {
    match cmd.commands {
        #[cfg(feature = "bootstrap")]
        Commands::Bootstrap(args) => super::bootstrap::bootstrap(args)?,
        Commands::MakeId(MakeId { command }) => make_id(command),
        Commands::HashPassword(HashPassword { password }) => hash_password(password)?,
    }

    Ok(())
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

fn hash_password(password: String) -> Result<(), anyhow::Error> {
    let hash = pic_store_auth::password::new_hash(password.as_str())?;
    println!("{hash}");
    Ok(())
}
