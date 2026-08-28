# Releasing bysqr

The version in `Cargo.toml`, the changelog heading and the Git tag are one
release identity. A release tagged as `vX.Y.Z` is accepted only when the crate
version is exactly `X.Y.Z` and `CHANGELOG.md` contains an `X.Y.Z` release
heading.

## Prepare a release

1. Update `Cargo.toml` to the new version and let Cargo update the root package
   version in `Cargo.lock`.
2. Move the relevant entries from `Unreleased` to a dated version heading in
   `CHANGELOG.md`.
3. Run the same core checks as CI:

   ```shell
   cargo fmt --all -- --check
   cargo clippy --locked --all-features --all-targets -- -D warnings
   cargo test --locked --all-features
   cargo package --locked
   ```

4. Merge the release commit to `main` and make sure its CI run is green.
5. Create and push an annotated tag on that exact commit:

   ```shell
   git switch main
   git pull --ff-only
   git status --short
   git tag -a vX.Y.Z -m "Release vX.Y.Z"
   git push origin vX.Y.Z
   ```

The tag starts `.github/workflows/release.yml`. The workflow validates the
release identity, builds and signs all native and WebAssembly artifacts, and
creates one GitHub release only after every build succeeds. When Trusted
Publishing is enabled, the crate is then published from the same tagged commit.

Published crates.io versions are immutable. If a published version is broken,
yank it and release a new patch version; never move or recreate a release tag.

## First crates.io release: 0.3.0

The first crates.io publication requires a crates.io API token because Trusted
Publishing can be configured only after the crate exists. Keep the GitHub
Actions repository variable `CRATES_IO_TRUSTED_PUBLISHING` unset, or set to
`false`, while creating `v0.3.0`. This makes the release workflow skip only its
crate publication job.

After the `v0.3.0` GitHub release succeeds:

1. Check out the exact release tag in a clean worktree:

   ```shell
   git fetch --tags origin
   git switch --detach v0.3.0
   git status --short
   cargo publish --dry-run --locked
   ```

2. Sign in to crates.io with the owner account, create a short-lived API token
   with only the required publishing permissions, authenticate Cargo without
   committing the token, then publish and remove the local credential:

   ```shell
   cargo login
   cargo publish --locked
   cargo logout
   ```

3. Confirm that `bysqr` version `0.3.0` and its docs.rs documentation are
   available, then revoke the one-time API token in the crates.io account
   settings.

## Enable automatic Trusted Publishing

Complete this once immediately after publishing `0.3.0`:

1. Open the `bysqr` crate settings on crates.io and add a GitHub Trusted
   Publisher with:

   - repository owner: `easydoklad`
   - repository name: `bysqr`
   - workflow filename: `release.yml`
   - environment: leave empty

2. In the GitHub repository settings, create the Actions repository variable
   `CRATES_IO_TRUSTED_PUBLISHING` with value `true`.

No crates.io token is stored in GitHub. For every later version tag, the
`rust-lang/crates-io-auth-action` exchanges the GitHub OIDC identity for a
short-lived crates.io token and `cargo publish --locked` runs automatically
after the GitHub release. There is no environment approval or other manual
release gate.

Do not enable the variable before the first manual publication and Trusted
Publisher configuration are complete. If an automated publish fails for a
transient reason, rerun only the failed job. If its package contents need a
source change, prepare a new patch release instead of moving the existing tag.
