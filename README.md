# pic-store

WIP Web service to convert images into multiple formats and sizes, and upload them to a CDN for usage with HTML `picture`, `srcset`, and so on.

## Roadmap

### v1

- [X] User/Team Authentication, Authorization, API Keys
- [X] Upload images, convert them into other sizes and formats, and upload to S3 or similar storage for hosting.
- [ ] Autogenerate `<picture>` tags for uploaded images.
- [ ] Generate blurhash or something similar for an image placeholder.

### v2

- Integrate the [social card generator](https://github.com/dimfeld/create-social-card) I wrote a while ago or other customization functionality.
- Vite plugin to download image info from the service and make it available to frontend code.
- Frontend components to facilitate drag-and-drop image upload

