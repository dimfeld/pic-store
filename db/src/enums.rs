use diesel_derive_enum::DbEnum;

#[derive(Debug, Clone, Copy, DbEnum)]
pub enum ImageFormat {
    Png,
    Jpg,
    Avif,
    Webp,
}

#[derive(Debug, Clone, Copy, DbEnum)]
pub enum BaseImageStatus {
    AwaitingUpload,
    Converting,
    Ready,
    QueuedForDelete,
    Deleting,
    Deleted,
}

impl Default for BaseImageStatus {
    fn default() -> Self {
        Self::AwaitingUpload
    }
}

#[derive(PartialEq, Eq, Copy, Clone, Debug, DbEnum)]
pub enum OutputImageStatus {
    Queued,
    Converting,
    Ready,
    QueuedForDelete,
    Deleted,
}

impl Default for OutputImageStatus {
    fn default() -> Self {
        Self::Queued
    }
}

#[derive(Copy, Clone, Debug, DbEnum)]
pub enum Permission {
    #[db_rename = "team:admin"]
    TeamAdmin,
    #[db_rename = "team:write"]
    TeamWrite,
    #[db_rename = "project:create"]
    ProjectCreate,
    #[db_rename = "project:write"]
    ProjectWrite,
    #[db_rename = "project:read"]
    ProjectRead,
    #[db_rename = "image:edit"]
    ImageEdit,
    #[db_rename = "image:create"]
    ImageCreate,
    #[db_rename = "conversion_profile:write"]
    ConversionProfileWrite,
    #[db_rename = "storage_location:write"]
    StorageLocationWrite,
}

impl std::fmt::Display for Permission {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let desc = match self {
            Self::TeamAdmin => "team:admin",
            Self::TeamWrite => "team:write",
            Self::ProjectCreate => "project:create",
            Self::ProjectRead => "project:read",
            Self::ProjectWrite => "project:write",
            Self::ImageCreate => "image:create",
            Self::ImageEdit => "image:edit",
            Self::ConversionProfileWrite => "conversion_profile:write",
            Self::StorageLocationWrite => "storage_location:write",
        };

        f.write_str(desc)
    }
}

impl Permission {
    /** Return true if this permission is linked to a project */
    pub fn requires_project(&self) -> bool {
        match self {
            Self::TeamWrite => false,
            Self::TeamAdmin => false,
            Self::ProjectCreate => false,
            Self::ProjectRead => true,
            Self::ProjectWrite => true,
            Self::ImageEdit => true,
            Self::ImageCreate => true,
            Self::ConversionProfileWrite => true,
            Self::StorageLocationWrite => true,
        }
    }
}
