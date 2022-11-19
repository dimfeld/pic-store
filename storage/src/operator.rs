use std::ops::Deref;

use bytes::Bytes;
use futures::stream::TryStreamExt;
use opendal::{Object, ObjectPart};
use std::io::Result;

pub struct Operator {
    pub operator: opendal::Operator,
    pub supports_multipart: bool,
}

impl Deref for Operator {
    type Target = opendal::Operator;

    fn deref(&self) -> &Self::Target {
        &self.operator
    }
}

impl Operator {
    pub async fn writer(&self, object: Object) -> Result<Writer> {
        let writer = match self.supports_multipart {
            true => Writer::Multipart(MultipartWriter::new(object).await?),
            false => Writer::Simple(SimpleWriter::new(object)),
        };

        Ok(writer)
    }
}

pub enum Writer {
    Multipart(MultipartWriter),
    Simple(SimpleWriter),
}

impl Writer {
    pub async fn complete(self) -> Result<Object> {
        match self {
            Self::Multipart(w) => w.complete().await,
            Self::Simple(w) => w.complete().await,
        }
    }

    pub async fn abort(self) -> Result<()> {
        match self {
            Self::Multipart(w) => w.abort().await,
            Self::Simple(w) => w.abort(),
        }
    }

    pub async fn add_part(&mut self, bytes: Bytes) -> Result<()> {
        match self {
            Self::Multipart(w) => w.add_part(bytes).await,
            Self::Simple(w) => w.add_part(bytes),
        }
    }
}

pub struct MultipartWriter {
    upload: opendal::ObjectMultipart,
    parts: Vec<ObjectPart>,
}

impl MultipartWriter {
    async fn new(object: Object) -> Result<Self> {
        Ok(MultipartWriter {
            upload: object.create_multipart().await?,
            parts: Vec::new(),
        })
    }

    pub async fn complete(self) -> Result<Object> {
        self.upload.complete(self.parts).await
    }

    pub async fn abort(&self) -> Result<()> {
        self.upload.abort().await
    }

    pub async fn add_part(&mut self, bytes: Bytes) -> std::io::Result<()> {
        self.parts
            .push(self.upload.write(self.parts.len(), bytes).await?);
        Ok(())
    }
}

pub struct SimpleWriter {
    object: Object,
    buffer: Vec<Bytes>,
}

impl SimpleWriter {
    fn new(object: Object) -> Self {
        SimpleWriter {
            object,
            buffer: Vec::new(),
        }
    }

    pub async fn complete(self) -> Result<Object> {
        let len = self.buffer.iter().map(|b| b.len() as u64).sum::<u64>();
        let stream = futures::stream::iter(self.buffer.into_iter().map(Ok)).into_async_read();
        self.object.write_from(len, stream).await?;
        Ok(self.object)
    }

    pub fn abort(&self) -> Result<()> {
        // Nothing to do here, we just ignore the data.
        Ok(())
    }

    pub fn add_part(&mut self, bytes: Bytes) -> Result<()> {
        self.buffer.push(bytes);
        Ok(())
    }
}
