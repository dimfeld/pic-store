# pic-store

Web service to convert images into multiple formats and sizes, and upload them to a CDN for usage with HTML `picture`, `srcset`, and so on.

While this is going to end up as a useful service, it is mostly designed to experiment with a few things:

- [Biscuit Auth tokens](https://www.biscuitsec.org/)
- The Axum web framework
- SQLite3 as a data store and Litestream replication/backup
- Evaluate [fly.io](https://fly.io) for deployment
- Vite plugins

## Roadmap

### v1

- Authentication through Biscuit tokens
- Upload images, convert them into other sizes and formats, and upload to S3 or similar storage for hosting.
- Autogenerate `<picture>` tags for uploaded images.
- Generate blurhash or something similar for an image placeholder.
- Integrate the [social card generator](https://github.com/dimfeld/create-social-card) I wrote a while ago
- Vite plugin to download image info from the service and make it available to frontend code.

### Beyond

Assuming this becomes truly useful, some other features that may arrive:

- Real user account system
- Web interface
- More scalability through queues and workers
