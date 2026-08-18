# Releasing FontNest

FontNest publishes unsigned Windows NSIS installers through GitHub Releases. The Windows
installer is not Authenticode-signed. Tauri updater signatures are still mandatory and verify
that every installed update was produced with the private FontNest updater key.

## One-time GitHub setup

Open the repository's **Settings > Secrets and variables > Actions** page and create these
repository secrets:

- `TAURI_SIGNING_PRIVATE_KEY`: the complete contents of
  `C:\Users\akari\.tauri\fontnest.key`.
- `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`: the password chosen when that key was generated.

To copy the private key without printing it in the terminal, run this in PowerShell:

```powershell
Get-Content -Raw -LiteralPath 'C:\Users\akari\.tauri\fontnest.key' | Set-Clipboard
```

Paste the clipboard contents into the first GitHub secret, then clear the clipboard. Never add
the private key or its password to the repository, an issue, a release, or application code.

Back up the private key and password in a second secure location. Installed copies of FontNest
will reject releases signed by a replacement key.

## Publish a release

1. Move every version file to the new version. `package.json` is the source of truth;
   `src-tauri/Cargo.toml`, `src-tauri/Cargo.lock` and `src-tauri/tauri.conf.json` follow it:

    ```powershell
    pnpm version:set 0.1.0
    ```

2. Update `CHANGELOG.md` by hand:
    - Rename the `## [Unreleased]` heading to `## [x.y.z] - YYYY-MM-DD` using today's date, and
      start a fresh empty `## [Unreleased]` section above it.
    - Add the version's comparison link at the bottom of the file, and point the `[Unreleased]`
      link at `v<x.y.z>...HEAD`.
    - FontNest shows these notes in **What's new**. It reads the copy on `main` first and falls
      back to the copy compiled into the build, so the changelog must be committed before the
      tag is pushed.
3. Run `pnpm verify`. It starts with `pnpm version:check`, which fails until every file above
   agrees, then runs the type, lint, test and Rust checks. The same version check runs in CI on
   every push, and again in the release workflow with the tag name.
4. Commit and push the release changes.
5. Create and push the matching version tag. For version `0.1.0`:

    ```powershell
    git tag v0.1.0
    git push origin v0.1.0
    ```

6. Watch the **Release FontNest** workflow in GitHub Actions.
7. Open the draft GitHub Release and confirm that it contains:
    - the NSIS `setup.exe` installer;
    - the matching `.sig` updater signature;
    - `latest.json`.
8. Review the generated release notes, then publish the draft.

Draft releases are intentionally invisible to the updater's `releases/latest` endpoint. A
release becomes available to installed applications only after the draft is published.

## Bootstrap and test the updater

The first published release containing the updater establishes the trusted public key and feed.
To verify a real update:

1. Publish and install FontNest `0.1.0`.
2. Run `pnpm version:set 0.1.1` and write the changelog section for it.
3. Publish `v0.1.1` with the same updater private key.
4. Launch the installed `0.1.0` build or use **Settings > Check for updates**.
5. Confirm that FontNest offers `0.1.1`, downloads it, verifies it, and closes for installation.

Never replace `latest.json` manually after the workflow has produced it. The release action uses
the generated installer signature and exact uploaded asset URL.
