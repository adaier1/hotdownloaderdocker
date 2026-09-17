FROM rust:1.85-bookworm

RUN curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh

ARG uid=1998
ARG gid=1998

RUN groupadd -g "$gid" builder \
    && useradd -u "$uid" -g "$gid" -m -d /h builder

USER builder

RUN curl -fsL https://get.pnpm.io/install.sh | ENV="$HOME/.bashrc" SHELL="$(which bash)" bash -
RUN git config --global --add safe.directory /a
ENV PATH="/h/.local/share/pnpm:$PATH"

WORKDIR /a
CMD ["/usr/bin/sleep", "infinity"]
