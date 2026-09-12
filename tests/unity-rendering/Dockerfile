# SPDX-License-Identifier: MIT OR Apache-2.0
# SPDX-FileCopyrightText: 2026 KooshaPari <kooshapari@gmail.com>
#
# Dockerfile for the Phase 6.2 real-Unity-Editor shader-rendering tests.
#
# Wraps the upstream unityci/editor image (Ubuntu 20.04 / focal, Unity 2022.3.20f1)
# so CI can run:
#
#   docker build -t phenotype-postfx-render tests/unity-rendering/
#   docker run --rm -v "$PWD:/project" -w /project \
#       phenotype-postfx-render \
#       Unity -batchmode -nographics -runTests \
#             -testPlatform EditMode \
#             -projectPath tests/unity-rendering \
#             -testResults tests/unity-rendering/results.xml
#
# The image is intentionally thin — the upstream image already provides the
# Unity binary, the IL2CPP backend, and the UPM client. We only add the
# `jq` utility for parsing the NUnit XML results inside the container, and
# the libgconf / libgdiplus shims that some Unity GL backends expect on
# headless hosts (see https://hub.docker.com/r/unityci/editor for upstream
# rationale).

FROM unityci/editor:ubuntu-2022.3.20f1-focal-2

LABEL org.opencontainers.image.title="phenotype-postfx-render" \
      org.opencontainers.image.description="Real-Unity-Editor test runner for the 7 phenotype-postfx passes (SSAO, SSGI, Bloom, ACES, Vignette, Chromatic, LUT)." \
      org.opencontainers.image.source="https://github.com/KooshaPari/phenotype-gfx" \
      org.opencontainers.image.licenses="MIT"

USER root

# Extra CLI tooling used by the workflow's result-parsing step.
RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        jq \
        ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Drop back to the unity user's writable home so UPM can create Library/.
USER unity

WORKDIR /project

ENTRYPOINT ["Unity"]