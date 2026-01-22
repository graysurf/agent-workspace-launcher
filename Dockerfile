ARG DOCKER_CLI_IMAGE="docker:27-cli"
FROM ${DOCKER_CLI_IMAGE} AS docker-cli

FROM ubuntu:24.04

ARG DEBIAN_FRONTEND=noninteractive

ARG ZSH_KIT_REPO="https://github.com/graysurf/zsh-kit.git"
ARG ZSH_KIT_REF="main"

ARG CODEX_KIT_REPO="https://github.com/graysurf/codex-kit.git"
ARG CODEX_KIT_REF="main"

LABEL org.opencontainers.image.source="https://github.com/graysurf/codex-workspace-launcher" \
  org.opencontainers.image.title="codex-workspace-launcher" \
  org.graysurf.zsh-kit.repo="$ZSH_KIT_REPO" \
  org.graysurf.zsh-kit.ref="$ZSH_KIT_REF" \
  org.graysurf.codex-kit.repo="$CODEX_KIT_REPO" \
  org.graysurf.codex-kit.ref="$CODEX_KIT_REF"

RUN apt-get update \
  && apt-get install -y --no-install-recommends \
    ca-certificates \
    curl \
    git \
    gnupg \
    rsync \
    zsh \
  && mkdir -p /root/.config \
  && rm -rf /var/lib/apt/lists/*

COPY --from=docker-cli /usr/local/bin/docker /usr/local/bin/docker

COPY bin/codex-workspace /usr/local/bin/codex-workspace
RUN chmod +x /usr/local/bin/codex-workspace

RUN git init -b main /opt/zsh-kit \
  && git -C /opt/zsh-kit remote add origin "$ZSH_KIT_REPO" \
  && git -C /opt/zsh-kit fetch --depth 1 origin "$ZSH_KIT_REF" \
  && git -C /opt/zsh-kit checkout --detach FETCH_HEAD \
  && git -C /opt/zsh-kit rev-parse HEAD >/opt/zsh-kit/.ref \
  && rm -rf /opt/zsh-kit/.git

RUN git init -b main /opt/codex-kit \
  && git -C /opt/codex-kit remote add origin "$CODEX_KIT_REPO" \
  && git -C /opt/codex-kit fetch --depth 1 origin "$CODEX_KIT_REF" \
  && git -C /opt/codex-kit checkout --detach FETCH_HEAD \
  && git -C /opt/codex-kit rev-parse HEAD >/opt/codex-kit/.ref \
  && rm -rf /opt/codex-kit/.git

ENV CODEX_WORKSPACE_LAUNCHER="/opt/codex-kit/docker/codex-env/bin/codex-workspace"

ENTRYPOINT ["codex-workspace"]
