FROM rust:1-buster AS builder

WORKDIR /usr/src/atasmart-exporter

# Build dependencies
RUN USER=root cargo init --bin /usr/src/atasmart-exporter
#COPY ./Cargo.lock ./Cargo.lock
COPY ./Cargo.toml ./Cargo.toml
RUN cargo build --release
RUN rm src/*.rs

RUN apt-get update && apt-get install -y libatasmart-dev

# Build actual project
COPY . .
RUN rm ./target/release/deps/atasmart_exporter*
RUN cargo install --path .

FROM debian:buster-slim
RUN apt-get update && apt-get install -y libatasmart4 && apt-get clean

COPY --from=builder /usr/src/atasmart-exporter/target/release/atasmart-exporter /app/bin/atasmart-exporter

CMD ["/app/bin/atasmart-exporter"]

