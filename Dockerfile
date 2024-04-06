FROM rust:1.76.0
WORKDIR /usr/src/chela
COPY . .
RUN cargo build -r
CMD ["./target/release/chela"]
EXPOSE 3000
