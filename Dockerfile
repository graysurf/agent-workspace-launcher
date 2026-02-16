ARG DOCKER_CLI_IMAGE="docker:27-cli"
FROM ${DOCKER_CLI_IMAGE} AS docker-cli

FROM ubuntu:24.04

ARG DEBIAN_FRONTEND=noninteractive

ARG ZSH_KIT_REPO="https://github.com/graysurf/zsh-kit.git"
ARG ZSH_KIT_REF="main"

ARG AGENT_KIT_REPO="https://github.com/graysurf/agent-kit.git"
ARG AGENT_KIT_REF="main"

LABEL org.opencontainers.image.source="https://github.com/graysurf/agent-workspace-launcher" \
  org.opencontainers.image.title="agent-workspace-launcher" \
  org.graysurf.zsh-kit.repo="$ZSH_KIT_REPO" \
  org.graysurf.zsh-kit.ref="$ZSH_KIT_REF" \
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

COPY bin/agent-workspace /usr/local/bin/agent-workspace
RUN chmod +x /usr/local/bin/agent-workspace

RUN mkdir -p /opt \
  && printf "%s\n" "$ZSH_KIT_REF" > /opt/zsh-kit.ref

RUN git init -b main /opt/agent-kit \
  && git -C /opt/agent-kit remote add origin "$AGENT_KIT_REPO" \
  && git -C /opt/agent-kit fetch --depth 1 origin "$AGENT_KIT_REF" \
  && git -C /opt/agent-kit checkout --detach FETCH_HEAD \
  && git -C /opt/agent-kit rev-parse HEAD >/opt/agent-kit/.ref \
  && rm -rf /opt/agent-kit/.git

ENV AGENT_WORKSPACE_LAUNCHER="/opt/agent-kit/docker/agent-env/bin/agent-workspace"

ENTRYPOINT ["agent-workspace"]
