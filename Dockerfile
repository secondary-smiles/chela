FROM rust:1.76.0 AS builder
WORKDIR /usr/src/chela
COPY . .
RUN cargo build -r

FROM gcr.io/distroless/cc-debian12
WORKDIR /usr/src/chela
COPY --from=builder /usr/src/chela/target/release/chela ./
CMD ["./chela"]
EXPOSE 3000
