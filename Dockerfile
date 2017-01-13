FROM lawliet89/debian-rust:1.14.0

WORKDIR /app/src

EXPOSE 3001

COPY ./ ./
RUN cargo build --release

CMD cargo run --release -- /app/config.toml
