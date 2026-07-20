# Build from the workspace root as the Docker context so the whole Cargo
# workspace resolves (the repo's .dockerignore -> .gitignore keeps target/ and
# node_modules/ out of the context):
#
#   docker build -f apps/seed-server/Dockerfile -t <registry>/modrinthium-seed-server:latest .
#
# Only the `seed-server` package and its dependency subgraph are compiled; the
# other workspace members are merely parsed for workspace resolution.

FROM rust:1.95-slim AS build
WORKDIR /src
COPY . .
RUN cargo build --release -p seed-server

FROM debian:stable-slim
RUN apt-get update \
	&& apt-get install -y --no-install-recommends ca-certificates \
	&& rm -rf /var/lib/apt/lists/*
COPY --from=build /src/target/release/seed-server /usr/local/bin/seed-server
ENV SEED_DATA_DIR=/data \
	SEED_BIND=0.0.0.0:8000
EXPOSE 8000
VOLUME ["/data"]
ENTRYPOINT ["seed-server"]
