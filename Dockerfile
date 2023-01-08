# Build base image using cargo-chef to optimize rebuilds.
FROM rust:1-slim-bullseye AS base

# Install ria dependencies.
RUN apt-get -y update && \
  apt-get install -y build-essential git curl wget \
  vim libssl-dev iputils-ping postgresql-client libasound2-dev
# Cache installation of sea-orm-cli at the base level.
RUN cargo install sea-orm-cli

# Now build the base chef image, which includes the above.
FROM base AS chef
WORKDIR /app
RUN cargo install cargo-chef

# First create a planner image to generate the dependency recipe.
FROM chef AS planner
COPY . .
RUN cargo chef prepare  --recipe-path recipe.json

# Then create a builder image to build all dependencies.
FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json
COPY . .
RUN cargo build --release

# Finally build the runtime image and tail the ria log.
FROM base AS runtime
WORKDIR /app
COPY --from=builder /app /app
