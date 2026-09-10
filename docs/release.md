# Releasing

[`.github/workflows/release.yml`](../.github/workflows/release.yml) runs on every push to `main`, and can also be started manually (**Actions → Release → Run workflow**). It follows the same flow as `just release`:

1. `cargo fmt --check` and `cargo test`
2. `cz bump` (version commit + tag + changelog)
3. Push commit and tags to `main`
4. `cargo publish` to crates.io
5. Create a GitHub Release

The job skips when the head commit message starts with `bump:`, or when Commitizen reports nothing to bump (exit code `21`).

Prefer the GitHub Action for routine releases. Use `just release` only for a local dry-run / emergency publish — otherwise you risk double-publishing.

## Allow third-party Actions

Workflows only need `actions/checkout` (Rust is installed with `rustup` on the runner, not via a third-party action).

If you see errors that actions are not allowed unless owned by `simone-sanfratello`:

1. Open **Settings → Actions → General**  
   https://github.com/simone-sanfratello/envro/settings/actions
2. Under **Actions permissions**, pick whichever option your UI shows that is **not** owner-only, for example:
   - **Allow all actions and reusable workflows**, or
   - **Allow OWNER, and select non-OWNER, actions and reusable workflows** (wording varies; on some personal accounts the nested allowlist is missing — use **Allow all**)
3. If an allowlist is available, enable **Allow actions created by GitHub** and/or add:

   ```text
   actions/*
   ```

4. Save

You do **not** need to allow `dtolnay/rust-toolchain` anymore.

## Secrets

| Secret | Required | Purpose |
| --- | --- | --- |
| `CARGO_REGISTRY_TOKEN` | yes | crates.io API token for `cargo publish` |
| `RELEASE_GITHUB_TOKEN` | recommended | Fine-grained PAT used to push the bump commit/tags (and create the release). Falls back to `GITHUB_TOKEN` if unset. |

Repo secrets UI:  
https://github.com/simone-sanfratello/envro/settings/secrets/actions

### `CARGO_REGISTRY_TOKEN`

1. Create a token at https://crates.io/settings/tokens
2. Scope: **publish-update** for crate `envro`
3. Store it as the repo secret `CARGO_REGISTRY_TOKEN`

### `RELEASE_GITHUB_TOKEN` (fine-grained PAT)

This is not a built-in GitHub token. Create a **fine-grained** personal access token, then store it as the repo secret `RELEASE_GITHUB_TOKEN`.

1. Open https://github.com/settings/personal-access-tokens/new  
   (Settings → Developer settings → Personal access tokens → Fine-grained tokens)
2. Set:
   - **Token name**: e.g. `envro-release`
   - **Expiration**: your choice
   - **Resource owner**: your user
   - **Repository access**: **Only select repositories** → `envro`
3. **Repository permissions**:
   - **Contents**: **Read and write** (push bump commit + tags; GitHub Releases)
   - **Metadata**: **Read-only** (required)
4. Generate the token and copy it
5. Add it as repo secret `RELEASE_GITHUB_TOKEN`

## Branch protection / bypass list

A PAT does **not** bypass branch protection by itself. The push is attributed to the token owner, so that user must be allowed to push to `main` (or listed as a bypass actor).

### Rulesets (current UI)

1. Repo → **Settings** → **Rules** → **Rulesets**  
   https://github.com/simone-sanfratello/envro/settings/rules
2. Open the ruleset that targets `main`
3. **Bypass list** → **Add bypass** → add your user (the PAT owner)
4. Allow bypass without a PR / required checks as needed

### Classic branch protection

1. Repo → **Settings** → **Branches**
2. Edit the rule for `main`
3. Enable **Allow specified actors to bypass required pull requests** (add yourself), and/or include yourself under **Restrict who can push to matching branches**
4. If **Do not allow bypassing the above settings** is on, even admins cannot bypass — turn it off or use the bypass list

If you do not see **Bypass list**, the repo is using classic branch protection.

## Configure secrets from `.env`

Put the values in a local `.env` (gitignored):

```env
CARGO_REGISTRY_TOKEN=...
RELEASE_GITHUB_TOKEN=...
```

Then:

```bash
set -a && source .env && set +a && ./scripts/setup-github-secrets.sh
```

`set -a` exports variables from `.env` so the script can set them via `gh secret set`.

Non-interactive alternatives:

```bash
CARGO_REGISTRY_TOKEN=... RELEASE_GITHUB_TOKEN=... ./scripts/setup-github-secrets.sh
SKIP_RELEASE_GITHUB_TOKEN=1 ./scripts/setup-github-secrets.sh   # crates.io only
```

Requires `gh` authenticated (`gh auth login`).

