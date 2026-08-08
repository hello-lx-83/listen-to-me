# Repository instructions

## Windows release workflow

When the user asks to package, publish, republish, or update the Windows installer, follow this workflow from the repository root.

1. Inspect the release scope before changing anything:
   - Run `git status --short --branch` and inspect relevant diffs and recent commits.
   - Read the version from `package.json`, `src-tauri/Cargo.toml`, and `src-tauri/tauri.conf.json`; keep all version declarations consistent.
   - Do not create an empty commit when the requested changes are already committed.
2. Keep generated binaries out of Git:
   - The installer belongs in GitHub Releases, not in the source tree's Git history.
   - Confirm `src-tauri/target/` and `src-tauri/target*/` remain ignored.
   - Never use `git add -f` for an installer or another generated build artifact.
3. Validate and package locally:
   - Run `pnpm check` and require all checks and Rust tests to pass.
   - Run `pnpm package:windows`. Do not add Vulkan, Ninja, CMake, or GPU feature flags unless the current source actually requires them.
   - The expected NSIS output is `src-tauri/target/release/bundle/nsis/Listen to Me_<version>_x64-setup.exe`.
   - Record its byte size, SHA-256, product version, and Authenticode status.
4. Publish source intentionally:
   - Commit only tracked source/configuration/documentation changes that belong to the release.
   - Push the intended commit to `origin/main` when the user explicitly requests direct publication to the main branch.
   - Verify `origin/main` resolves to the intended local commit.
5. Publish the installer as a GitHub Release asset:
   - Prefer a new patch version and a new tag for code changes after a published release (for example, `0.1.0` to `0.1.1`).
   - Create `v<version>` at the exact commit used for the build and push the tag.
   - Create the GitHub Release and upload the NSIS installer as its asset.
   - If the user explicitly asks to replace an existing release for the same version, first confirm the release/tag target, then move the tag if necessary and upload with `gh release upload v<version> <installer> --clobber`. Do not silently rewrite a published tag.
   - Update the release notes with the current SHA-256 and the unsigned/SmartScreen notice when applicable.
6. Verify the remote result before reporting success:
   - Use `gh release view v<version> --json tagName,url,assets` (or the GitHub API) after upload.
   - Require the remote asset state to be `uploaded` and its size and `digest` to match the local file exactly.
   - Verify both `refs/heads/main` and `refs/tags/v<version>` point to the intended commit.
   - Report the Release page, direct asset URL, commit, size, and SHA-256.

GitHub's repository home page displays the Release's original publication time. Replacing an asset does not refresh that card's relative time, so never use the home-page timestamp as proof that an upload is stale; verify the asset digest and asset creation/update timestamps instead.

## GitHub tooling

- Use an authenticated GitHub CLI (`gh`) for Release asset upload when available.
- If `gh` is missing, do not claim that the installer was published. Either obtain explicit approval to use the official portable CLI or stop and give the user the manual Release upload path.
- A successful `git push` publishes source commits and tags only; it does not upload the installer. Release asset upload and remote digest verification are separate required steps.
