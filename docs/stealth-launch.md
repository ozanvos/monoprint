# Desktop launch

The `Desktop release` workflow builds macOS Apple Silicon, Windows x64 and Linux
x64 at one commit. It reuses the existing Sparkle and Velopack packagers. Intel
Macs and ARM Windows/Linux are not included in this initial matrix. Linux builds
require glibc 2.39 (Ubuntu 24.04 or newer) and may need FUSE 2.

Nothing publishes on a push or tag. `publicPublishing` in `release/config.json`
permits publication; a manual dispatch with `publish: true`, `signed: true` and
an existing matching version tag authorizes a particular release. The older
`Release candidate` workflow remains available for CLI/npm preparation and cannot
replace the desktop channel's latest release.

## One-time signing and hosting configuration

Use GitHub environment secrets/variables; never commit private signing material.
The repository must be public before publishing downloads or build attestations.

| Environment | Setting | Purpose |
| --- | --- | --- |
| `apple-signing` | Secret `MACOS_CERTIFICATE_BASE64` | Base64 PKCS#12 export of a **Developer ID Application** certificate and its private key |
| `apple-signing` | Secret `MACOS_CERTIFICATE_PASSWORD` | PKCS#12 password |
| `apple-signing` | Secret `MACOS_SIGN_IDENTITY` | Full Developer ID Application identity name |
| `apple-signing` | Secrets `APPLE_API_KEY_BASE64`, `APPLE_API_KEY_ID`, `APPLE_API_ISSUER_ID` | App Store Connect API key with notarization access |
| `apple-signing` | Secret `SPARKLE_PRIVATE_KEY` | Text exported by Sparkle's `generate_keys -x`; preserve it across releases |
| `apple-signing` | Variable `MONOPRINT_SPARKLE_PUBLIC_KEY` | Corresponding base64 Ed25519 public key |
| `windows-signing` | Secrets `AZURE_TENANT_ID`, `AZURE_CLIENT_ID` | OIDC identity with Certificate Profile Signer access to the Monoprint Azure signing profile |
| `windows-signing` | Variable `MONOPRINT_WINDOWS_SIGNING_METADATA` | JSON containing `Endpoint`, `CodeSigningAccountName`, `CertificateProfileName` |
| `website` | Secret `CLOUDFLARE_API_TOKEN` | Individual Workers Editor, scoped only to the existing `monoprint-site` Worker |
| `website` | Variable `CLOUDFLARE_ACCOUNT_ID` | Account hosting `monoprint.7ea.st` |

`public-release` is the publication environment. Configure its reviewers and
branch/tag restrictions according to the repository's release approval policy.
For signing environments, permit only trusted release refs.

Website deployment uploads a uniquely tagged Worker version and deploys that
exact version to 100% of traffic. It preserves existing routes and custom domains,
so the CI token does not need zone or account-wide Worker permissions. Domain
changes remain a separate infrastructure operation. Rotate the token before its
configured expiration.

The Sparkle key is configured in `apple-signing` and retained in the macOS login
Keychain under account `app.monoprint.studio`. Its public key is also pinned in
`release/updates.json`; distribution assembly rejects a different embedded key.
Back up the private key securely and preserve it for future updates.

Apple Development and Mac App Store distribution certificates do **not** replace
Developer ID Application signing for direct downloads. Apple notarization and
Sparkle update signing are separate requirements. The workflow stores private
material only in temporary files/keychains and removes it after use.

Windows uses Velopack's `--azureTrustedSignFile` support on the native Windows
runner. Set up a public-trust Azure signing account/profile and complete identity
validation before attempting a signed build. This repository does not provision
or purchase that service. Other providers remain possible through the packager's
existing `--sign-template` option, but require a corresponding CI setup.
See [Velopack signing](https://docs.velopack.io/packaging/signing).

The Entra app's federated credential must use issuer
`https://token.actions.githubusercontent.com`, audience
`api://AzureADTokenExchange`, and subject
`repo:ozanvos@111265/monoprint@1430299394:environment:windows-signing`.
This repository has GitHub's immutable OIDC subjects enabled. Confirm the
current prefix with `gh api repos/ozanvos/monoprint/actions/oidc/customization/sub`
before configuring federation; the older name-only subject will not match.
Grant its service principal
Artifact Signing Certificate Profile Signer on the Monoprint profile only.
The workflow logs in through OIDC immediately before packaging; the signing
library uses that Azure CLI session. No Azure client password is required.
The `windows-signing` environment permits only `main`, `codex/stealth-launch`,
and `v*` tags. Remove the launch branch allowance after qualification.

The initial Azure signing account is approved for one billing month only.
Billing continues until the account is deleted; no automatic cancellation is
configured. Deletion stops future signing but does not invalidate existing
signatures. Do not assume subsequent billing periods are authorized.
See [Microsoft's billing and unenrollment FAQ](https://learn.microsoft.com/en-us/azure/artifact-signing/faq#unenroll-from-the-artifact-signing-service).

The business verification homepage is maintained separately in `company-site/`
and deployed to the `seven-east-site` Worker at `7ea.st`. It has no build step
and does not use the Monoprint website CI token.

Confirm publisher eligibility before purchasing: Microsoft currently accepts
organizations in the US, Canada, EU and UK, but individual developers only in the
US and Canada. A Netherlands individual publisher needs another signing provider.
See [Microsoft's signing options](https://learn.microsoft.com/en-us/windows/apps/package-and-deploy/code-signing-options).

Linux assets, and every other released asset, receive a Sigstore build attestation
through GitHub OIDC. The release includes `provenance.sigstore.json` and
`SHA256SUMS`. Verify a downloaded file with:

```sh
gh attestation verify Monoprint-linux-x64.AppImage --repo ozanvos/monoprint
```

Use the actual downloaded filename. This proves build provenance; Linux's
Velopack updater checks feed hashes over HTTPS, not the Sigstore attestation.
See [GitHub artifact attestations](https://docs.github.com/en/actions/how-tos/secure-your-work/use-artifact-attestations/use-artifact-attestations).

## Prepare, qualify, publish

1. Merge reviewed changes; keep the package versions/locks and release notes in
   sync. Run `node scripts/release.mjs check`.
2. Dispatch `Desktop release` from the intended commit with publishing off.
   Unsigned candidates test packaging without credentials; signed candidates
   additionally test notarization, Authenticode and Sparkle signatures. Only
   signed candidates pass distribution validation.
3. Qualify the signed artifacts on native machines, including fresh install,
   launch, saved drafts, exports, manual/automatic updates, offline retry,
   restart and uninstall. Verify two consecutive versions upgrade correctly.
   Before a public channel exists, use isolated test feeds with the standalone
   packagers; the public workflow requires the committed production feed URLs.
4. Tag the qualified source as `vVERSION`, push the tag, and dispatch from that
   tag with `signed: true`, `publish: true`, and `tag: vVERSION`.
5. CI rebuilds and verifies all platforms, signs provenance, uploads every asset
   into a draft, then publishes the complete release as latest. Failures before
   promotion leave the previous latest release active. A failed draft must be
   inspected and removed before retrying; the workflow never overwrites assets.
6. CI checks public GitHub asset sizes and digests against `release.json`, then
   builds, browser-tests and deploys the website. If website deployment fails,
   the release remains valid; rerun the `Website` workflow with publishing on.

The macOS archive is Developer ID signed, notarized and stapled before Sparkle
signs the ZIP. Windows signing runs inside Velopack so the application, updater
and installer are signed at the right packaging stages. Native package checks
run before receipts can claim release eligibility. Assembly rechecks all receipt
commits, source hashes, versions and artifact hashes, and validates both updater
formats against the shipping bytes.

## Update channel and website

`release/updates.json` points at GitHub's `releases/latest/download/` directory.
macOS uses `appcast-macos-arm64.xml`; Windows and Linux use separate
`releases.win-x64.json` and `releases.linux-x64.json` feeds. Sparkle enclosures and
website download links use immutable version-tag URLs. Keep old releases and
packages available. Do not mark a release without all three platform feeds as
latest, or publish a lower version over the channel.

The website only enables downloads when `MONOPRINT_DOWNLOAD_MANIFEST` names the
output of `scripts/verify-desktop-release.mjs`. That command checks the actual
public stable release and all its asset digests; a candidate, draft, prerelease,
missing installer or incomplete feed cannot enable the links. Local previews
without that environment variable show the existing disabled download buttons.

```sh
node scripts/verify-desktop-release.mjs --out output/published.json
MONOPRINT_DOWNLOAD_MANIFEST="$PWD/output/published.json" npm run build --prefix website
```

Do not serve synthetic test manifests or preview output as a production download
page. The workflow's public download check requires no repository token, which
also confirms ordinary visitors can access the release metadata.
