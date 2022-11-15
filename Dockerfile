# Build base image using cargo-chef to optimize rebuilds.
FROM ubuntu:latest AS base

ENV DEBIAN_FRONTEND noninteractive
# Add cargo to the path.
ENV PATH="/root/.cargo/bin:${PATH}"

# Install ria dependencies.
RUN apt-get -y update && \
  apt-get install -y build-essential git curl wget \
  vim libssl-dev iputils-ping postgresql-client \
  libgstreamer1.0-dev libgstreamer-plugins-base1.0-dev \
  libgstreamer-plugins-bad1.0-dev gstreamer1.0-plugins-base \
  gstreamer1.0-plugins-good gstreamer1.0-plugins-bad \
  gstreamer1.0-plugins-ugly gstreamer1.0-libav gstreamer1.0-tools \
  gstreamer1.0-x gstreamer1.0-alsa gstreamer1.0-gl gstreamer1.0-gtk3 \
  gstreamer1.0-qt5 gstreamer1.0-pulseaudio

# Install Rust and associated tools.
RUN curl https://sh.rustup.rs -sSf | bash -s -- -y

# Cache installation of sea-orm-cli at the base level.
RUN cargo install sea-orm-cli

# Now build the base chef image, which includes the above.
FROM base AS chef
WORKDIR /app
RUN apt-get install -y build-essential
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
CMD /usr/bin/tail -F /app/ria.log
