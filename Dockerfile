FROM rust:1.79.0

WORKDIR /app

COPY . .

RUN cargo build --release

EXPOSE 3000

CMD ["cargo", "run", "--release"]