use crate::provider::Provider;

impl Provider {
    pub async fn create_presigned_url(&self) {
        match self {
            Self::Local => {}
            Self::S3 { client, .. } => {}
        }
    }
}
