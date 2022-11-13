FROM ubuntu:latest

# Don't prompt for questions!
ENV DEBIAN_FRONTEND noninteractive

# Install dependencies.
RUN apt-get -y update && \
  apt-get -y upgrade && \
  apt-get install -y coreutils sudo build-essential git curl wget \
  vim libssl-dev iputils-ping postgresql-client \
  libgstreamer1.0-dev libgstreamer-plugins-base1.0-dev \
  libgstreamer-plugins-bad1.0-dev gstreamer1.0-plugins-base \
  gstreamer1.0-plugins-good gstreamer1.0-plugins-bad \
  gstreamer1.0-plugins-ugly gstreamer1.0-libav gstreamer1.0-tools \
  gstreamer1.0-x gstreamer1.0-alsa gstreamer1.0-gl gstreamer1.0-gtk3 \
  gstreamer1.0-qt5 gstreamer1.0-pulseaudio

# Install Rust and associated tools.
RUN curl https://sh.rustup.rs -sSf | bash -s -- -y

# Add cargo the path.
ENV PATH="/root/.cargo/bin:${PATH}"

# Install sea-orm-cli.
RUN cargo install sea-orm-cli

# Build Ria
WORKDIR /app
COPY . .
RUN touch /app/ria.log && cargo build --release

CMD /usr/bin/tail -f /app/ria.log