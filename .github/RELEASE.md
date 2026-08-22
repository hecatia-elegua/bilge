# Publish to crates.io

Manual trigger on Github. The first run should always be a dry-run.

For 0.x bumps, minor versions are for breaking changes: `0.3.0 -> 0.4.0`.
For 1.x bumps, it works as usual with semantic versioning.

No crates.io API tokens. Publishing uses [trusted publishing](https://crates.io/docs/trusted-publishing) (OIDC, ~30 min token, revoked when the job ends).

## One-time setup was

1. GitHub/Settings/Environments/New named exactly `crates-io`.
   Enable required reviewers, limit deployment branches to `main`.
2. On crates.io, for both [`bilge`](https://crates.io/crates/bilge/settings) and [`bilge-impl`](https://crates.io/crates/bilge-impl/settings):
   - trusted publishing: GitHub
   - repository: `hecatia-elegua/bilge`
   - workflow filename: `release.yml`
   - environment: `crates-io`
   - enable **Trusted Publishing Only**
3. `main` requires pull requests, so `GITHUB_TOKEN` cannot push the release commit.
   Add repo secret `RELEASE_PUSH_TOKEN` with the value being a PAT (configured in Profile/Developer-Settings/Fine-grained-token with Repository Content > Read/Write access on default branch).

## Releasing the next version

1. Changelog for the version you are about to bump to must exist, e.g. `## [0.4.0] - Unreleased` with at least one `- ` bullet.
2. Github/Actions/Release/Run → `bump: patch/minor/major`, `dry_run: true`.
3. If that is green, run again with `dry_run: false`. Approve the `crates-io` environment prompt.

The workflow prepares once, tests and dry-runs that tree, then publishes those same files. If a crate version is already on crates.io, the publish job skips it (recovery after a partial run).

If crates.io succeeds but `git push` fails, re-run the same bump with `dry_run: false`. Already-published crates are skipped so the job can finish the git tag and GitHub Release. If git also succeeded and only the GitHub Release step failed, create that release by hand rather than re-running (a second run would bump again).
