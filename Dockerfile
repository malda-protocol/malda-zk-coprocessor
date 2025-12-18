FROM ubuntu:24.04

WORKDIR /app

# Install base dependencies.
ARG DEBIAN_FRONTEND=noninteractive
RUN apt update && apt install --yes --force-yes curl gcc

# Install Rust.
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
ENV PATH="/root/.cargo/bin:${PATH}"

# Install Risc0.
RUN curl -L https://risczero.com/install | bash
ENV PATH="/root/.risc0/bin:${PATH}"
RUN rzup install && rzup install r0vm 3.0.4

# Copy the project files.
COPY . /app

# Compile Risc0 guest.
RUN cargo build -p methods --release --locked

# Copy the entrypoint script (prints guest info).
RUN chmod +x /app/entrypoint.sh

# Set the entrypoint.
ENTRYPOINT ["/app/entrypoint.sh"]
