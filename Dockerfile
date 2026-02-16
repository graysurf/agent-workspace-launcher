ARG DOCKER_CLI_IMAGE="docker:27-cli"
FROM ${DOCKER_CLI_IMAGE} AS docker-cli

FROM rust:1-slim AS rust-builder
WORKDIR /workspace

COPY Cargo.toml Cargo.lock rust-toolchain.toml rustfmt.toml ./
COPY crates/agent-workspace/Cargo.toml crates/agent-workspace/Cargo.toml
COPY crates/agent-workspace/src crates/agent-workspace/src

RUN cargo build --release --locked -p agent-workspace

FROM ubuntu:24.04

ARG DEBIAN_FRONTEND=noninteractive

ARG AGENT_KIT_REPO="https://github.com/graysurf/agent-kit.git"
ARG AGENT_KIT_REF=""

LABEL org.opencontainers.image.source="https://github.com/graysurf/agent-workspace-launcher" \
  org.opencontainers.image.title="agent-workspace-launcher" \
  org.graysurf.agent-kit.repo="$AGENT_KIT_REPO" \
  org.graysurf.agent-kit.ref="$AGENT_KIT_REF"

RUN apt-get update \
  && apt-get install -y --no-install-recommends \
    ca-certificates \
    curl \
    git \
    gnupg \
    jq \
    rsync \
    zsh \
  && mkdir -p /root/.config \
  && rm -rf /var/lib/apt/lists/*

COPY --from=docker-cli /usr/local/bin/docker /usr/local/bin/docker
COPY --from=rust-builder /workspace/target/release/agent-workspace /usr/local/bin/agent-workspace
COPY VERSIONS.env /tmp/VERSIONS.env

RUN git init -b main /opt/agent-kit \
  && resolved_ref="$AGENT_KIT_REF" \
  && if [ -z "$resolved_ref" ]; then resolved_ref="$(awk -F= '/^AGENT_KIT_REF=/{print $2}' /tmp/VERSIONS.env | tail -n 1 | tr -d '\r')"; fi \
  && if [ -z "$resolved_ref" ]; then echo "error: AGENT_KIT_REF missing (build-arg and VERSIONS.env)" >&2; exit 1; fi \
  && git -C /opt/agent-kit remote add origin "$AGENT_KIT_REPO" \
  && git -C /opt/agent-kit fetch --depth 1 origin "$resolved_ref" \
  && git -C /opt/agent-kit checkout --detach FETCH_HEAD \
  && sed -i 's/chown -R codex:codex/chown -R agent:agent/g' /opt/agent-kit/docker/agent-env/bin/agent-workspace \
  && git -C /opt/agent-kit rev-parse HEAD >/opt/agent-kit/.ref \
  && rm -rf /opt/agent-kit/.git /tmp/VERSIONS.env

ENTRYPOINT ["agent-workspace"]
