#!/bin/bash

if [[ ! "$GITHUB_REF" =~ ^refs/tags/ ]]; then
    echo "skip package publish, pack only."
    pnpm pack
    exit 0
fi

pnpm config set -- '//git.um-react.app/api/packages/um/npm/:_authToken' "${NPM_TOKEN}"
pnpm publish --access=public --no-git-checks
