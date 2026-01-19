FROM ubuntu:24.04

ARG DEBIAN_FRONTEND=noninteractive

ARG ZSH_KIT_REPO="https://github.com/graysurf/zsh-kit.git"
ARG ZSH_KIT_REF="main"

ARG CODEX_KIT_REPO="https://github.com/graysurf/codex-kit.git"
ARG CODEX_KIT_REF="main"

RUN apt-get update \
  && apt-get install -y --no-install-recommends \
    ca-certificates \
    curl \
    docker.io \
    git \
    openssl \
    python3 \
    rsync \
    zsh \
  && rm -rf /var/lib/apt/lists/*

COPY bin/codex-workspace /usr/local/bin/codex-workspace
RUN chmod +x /usr/local/bin/codex-workspace

RUN git init -b main /opt/zsh-kit \
  && git -C /opt/zsh-kit remote add origin "$ZSH_KIT_REPO" \
  && git -C /opt/zsh-kit fetch --depth 1 origin "$ZSH_KIT_REF" \
  && git -C /opt/zsh-kit checkout --detach FETCH_HEAD \
  && rm -rf /opt/zsh-kit/.git

RUN git init -b main /opt/codex-kit \
  && git -C /opt/codex-kit remote add origin "$CODEX_KIT_REPO" \
  && git -C /opt/codex-kit fetch --depth 1 origin "$CODEX_KIT_REF" \
  && git -C /opt/codex-kit checkout --detach FETCH_HEAD \
  && rm -rf /opt/codex-kit/.git

ENV CODEX_WORKSPACE_LAUNCHER="/opt/codex-kit/docker/codex-env/bin/codex-workspace"

ENTRYPOINT ["codex-workspace"]
