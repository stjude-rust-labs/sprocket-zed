# Release Process

1. Bump the version in `extension.toml`.
2. Commit with the message `release: bumps the version to vX.Y.Z`.
3. Tag the commit: `git tag vX.Y.Z && git push --tags`.
4. Create a GitHub release for the tag.
