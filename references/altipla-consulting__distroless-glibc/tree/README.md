
# distroless-glibc

Distroless Docker image for static and glibc binaries.


## Usage

Use it as the base image in your Dockerfile:

```dockerfile
FROM ghcr.io/altipla-consulting/distroless-glibc:latest

WORKDIR /opt/ac
COPY tmp/bin/foo .

CMD ["/opt/ac/foo"]
```
