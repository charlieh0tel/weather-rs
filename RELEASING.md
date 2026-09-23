# Releasing

Pushing a `vX.Y.Z` tag builds the Debian packages, publishes the GitHub
release, and updates the APT repository.

## Steps

1. Make sure `main` is green in CI and that `cargo audit` is clean. An
   audit failure blocks the release.
2. Bump `version` in `Cargo.toml` and refresh the lockfile:

   ```sh
   cargo build
   cargo fmt --check && cargo clippy --all-targets && cargo test
   ```

3. Commit the bump with the version as the message, tag it, and push both:

   ```sh
   git commit -am vX.Y.Z
   git tag vX.Y.Z
   git push origin main vX.Y.Z
   ```

The package version comes from `Cargo.toml`, not from the tag, so the two
must match.

Use a patch bump for dependency-only or fix-only releases.

## What the tag triggers

`.github/workflows/build.yml`, via the shared
`charlieh0tel/deb-workflows` `rust-build-deb.yml`:

- `build-deb`: builds and tests the amd64 and arm64 `.deb` packages.
- `audit`: runs `cargo audit`.
- `release`: creates the GitHub release with both `.deb` files and
  generated release notes.
- `trigger-apt-repo`: dispatches `update-repo.yml` in
  `charlieh0tel/apt-repo`, which rebuilds the repository and deploys it
  to GitHub Pages. This uses the `APT_REPO_TOKEN` secret.

## Checking it

```sh
gh run list --limit 4
gh release view vX.Y.Z
gh run list -R charlieh0tel/apt-repo --limit 2
```

The release is done when the apt-repo `pages build and deployment` run
succeeds.
