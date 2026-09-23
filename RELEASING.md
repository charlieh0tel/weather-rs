# Releasing

```sh
# bump version in Cargo.toml
cargo build
git commit -am vX.Y.Z
git tag vX.Y.Z
git push origin main vX.Y.Z
```

See [deb-workflows](https://github.com/charlieh0tel/deb-workflows) for what
the tag triggers.
