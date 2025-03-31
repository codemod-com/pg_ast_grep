FROM rust:1.85-bookworm

# Install PostgreSQL 16 and development packages
RUN apt-get update && \
    apt-get install -y \
    pkg-config \
    libssl-dev \
    git \
    build-essential \
    libreadline-dev \
    zlib1g-dev \
    flex \
    bison \
    libxml2-dev \
    libxslt-dev \
    libxml2-utils \
    xsltproc \
    ccache \
    libclang-dev \
    && rm -rf /var/lib/apt/lists/*

# Create a non-root user
RUN useradd -m -s /bin/bash developer

# Switch to non-root user
USER developer

# Install cargo-pgrx
RUN cargo install cargo-pgrx --locked

# Initialize pgrx for the user
RUN cargo pgrx init

# Set up working directory
WORKDIR /app

CMD ["bash"] 