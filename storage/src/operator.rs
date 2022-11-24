use bytes::Bytes;
use object_store::{path::Path, GetResult, MultipartId, ObjectStore};
use tokio::io::AsyncWrite;
use tracing::instrument;

use crate::error::{Error, Result};

pub struct Operator {
    pub operator: Box<dyn ObjectStore>,
    pub supports_multipart: bool,
    pub path_prefix: Option<Path>,
}

impl Operator {
    fn make_full_path(&self, location: &str) -> Path {
        match &self.path_prefix {
            Some(prefix) => prefix.child(location),
            None => Path::from(location),
        }
    }

    #[instrument(skip(self))]
    pub async fn get(&self, location: &str) -> Result<GetResult> {
        let p = self.make_full_path(location);
        self.operator.get(&p).await.map_err(Error::from)
    }

    #[instrument(skip(self))]
    pub async fn put(&self, location: &str, bytes: Bytes) -> Result<()> {
        let p = self.make_full_path(location);
        self.operator.put(&p, bytes).await.map_err(Error::from)
    }

    #[instrument(skip(self))]
    pub async fn put_multipart(
        &self,
        location: &str,
    ) -> Result<(MultipartId, Box<dyn AsyncWrite + Unpin + Send>)> {
        let p = self.make_full_path(location);
        self.operator.put_multipart(&p).await.map_err(Error::from)
    }

    #[instrument(skip(self))]
    pub async fn abort_multipart(&self, location: &str, id: &MultipartId) -> Result<()> {
        let p = self.make_full_path(location);
        self.operator
            .abort_multipart(&p, id)
            .await
            .map_err(Error::from)
    }
}
