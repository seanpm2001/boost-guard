FROM rust:1 AS chef
RUN cargo install cargo-chef
WORKDIR /app

FROM chef AS planner
COPY ./Cargo.toml ./Cargo.lock ./
COPY ./src ./src
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /app/recipe.json .
RUN cargo chef cook --release --recipe-path recipe.json
COPY . .
RUN cargo build --release --bin boost-guard

FROM debian:bookworm-slim AS runtime
RUN apt-get update && apt-get install -y ca-certificates libssl-dev libssl3
WORKDIR /app
COPY --from=builder /app/target/release/boost-guard /usr/local/bin

# NOTE: This will be removed, it's a random private key taken from the internet.
ENV PRIVATE_KEY "0xafdfd9c3d2095ef696594f6cedcae59e72dcd697e2a7521b1578140422a4f890"

ENV HUB_URL "https://hub.snapshot.org/graphql"
ENV TESTNET_HUB_URL "https://testnet.hub.snapshot.org/graphql"
ENV SEPOLIA_SUBGRAPH_URL "https://api.studio.thegraph.com/query/23545/boost-sepolia/version/latest"
ENV MAINNET_SUBGRAPH_URL "https://api.studio.thegraph.com/query/23545/boost/version/latest"
ENV BOOST_NAME "boost"
ENV BOOST_VERSION "0.1.0"
ENV VERIFYING_CONTRACT "0xc8Ae580637bf91b7E2c0A8cf369Fb24e0253cA5a"

ENTRYPOINT ["/usr/local/bin/boost-guard"]