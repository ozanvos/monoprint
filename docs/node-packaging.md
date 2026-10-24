# Node package builds

From `bindings/node`, with Rust 1.97+, a C compiler, Node.js 22+, and the checked-in Cargo/npm locks:

```sh
npm ci
npm run build
npm test
npm run test:types
npm run test:package
```

The build produces both the Node binding and the existing Rust CLI for the selected target. Each platform package contains both binaries; the main package exposes `monoprint` through its `bin` launcher and keeps the Node.js API exports.

The last command packs the host binaries and installs the tarballs into a clean, relocated consumer. It tests offline `npm exec` command discovery, CLI rendering and exit codes, ESM, CommonJS, TypeScript resolution and a server bundle with install scripts disabled. These local-only artifacts cannot be published by the release script.

The `npm-packages.yml` workflow builds and tests each target, collects the exact binaries and their source/checksum receipts, then uploads a complete `npm-release` artifact. It does not publish.

## GitHub Actions publication

The **Release candidate** workflow (`release.yml`) supports npm-only releases through GitHub OIDC. It uses the existing `public-release` environment, runs the source checks and all five native package tests, and publishes the verified tarballs with provenance. No npm token secret is needed. A push or tag never publishes by itself.

For each release, update all package versions and the release notes, merge the source, and create its `vVERSION` tag. Dispatch from that tag:

```sh
gh workflow run release.yml --ref vVERSION -f npm_only=true -f publish=true -f tag=vVERSION
```

To check the live OIDC relationships without building or uploading packages:

```sh
gh workflow run release.yml --ref main -f npm_only=true -f publish=false
```

Each of the six npm packages must trust **ozanvos/monoprint**, workflow **release.yml**, environment **public-release**, with direct publishing allowed. Configure this once using npm 11.15+ and an interactive maintainer login:

```sh
npm trust github PACKAGE --repo ozanvos/monoprint --file release.yml --environment public-release --allow-publish --yes
```

Replace `PACKAGE` with `monoprint` and each platform package listed in `bindings/node/targets.json`. Inspect existing configurations with `npm trust list PACKAGE`; preserve unrelated publishers. Setup requires npm 2FA. A new configuration must complete its first successful publish within two days or npm expires it. See [npm trusted publishing](https://docs.npmjs.com/trusted-publishers/).

The release waits up to 30 minutes for npm's publish-time scan, checks the public archive integrity against the CI artifacts, then installs the registry package and tests the CLI and Node API. Availability delay does not mean the upload failed; do not rebuild or replace an already accepted version.

## Manual recovery publication

For an explicitly authorized local publication:

1. Set the final npm package name and optional repository metadata in `package.json`. The default name is `monoprint`; ownership/availability must be checked for it and all five platform names before publication. Keep the npm and `monoprint-node` Cargo versions equal, update the npm lock, and commit the intended source.
2. Run the workflow and download its `npm-release` artifact into a fresh directory. Every declared target must pass its native runtime tests. The artifact contains `manifest.json` and `tarballs/`.
3. From the matching source checkout, verify the artifact without contacting npm:

   ```sh
   npm run publish:packages -- --from /path/to/npm-release --dry-run
   ```

4. Public publication is enabled in `release/config.json`, but each invocation
   still requires approval. Authenticate the npm account that owns all six names
   with `npm login`, then publish the validated artifacts from that same checkout:

   ```sh
   MONOPRINT_PUBLIC_PUBLISH=approved npm run publish:packages -- --from /path/to/npm-release --publish
   ```

   Complete npm's authentication prompts as requested. Do not put credentials in
   Git or command arguments. `--provenance` is only for a supported CI publisher
   with OIDC configured; a local first publication does not carry npm provenance.

Platform packages publish first and the main package last. The script uses `--ignore-scripts`; neither packing nor a dry run can trigger hidden publication. Versions on npm are immutable and multi-package publication is not atomic. If publication stops partway, inventory the published versions, compare their archive integrity to the saved manifest, and publish only the remaining unchanged tarballs with `npm publish /path/to/file.tgz --access public --ignore-scripts`. Do not rebuild different binaries under the same version. Optional `--tag next` selects a prerelease dist-tag; prerelease versions default to `next`.

Without `--publish`, the publisher only validates, even if `--dry-run` is omitted.
Receipts bind both native binaries and tarballs to one source commit and input hash;
mixed revisions, dirty release builds and altered packed metadata are rejected.
The publisher also checks the main package's executable entry and launcher.
The already-published `0.1.0` has no CLI entry; ship this addition in a new
version rather than attempting to republish `0.1.0`.
See [release packaging](release.md) for workflow configuration and npm OIDC setup.

To assemble already-built release artifacts yourself, use `npm run package -- --artifacts /path/to/native-artifacts --out /fresh/output`. It refuses missing, duplicate, stale, wrong-version, or altered native artifacts. `--local` creates an explicitly non-release subset for local testing. Output paths must be fresh; packaging never deletes an existing caller-selected directory.

Every tarball includes project/third-party notices, font licenses, MPL grammar sources, and unchanged source archives for Giallo and MPL-covered Rust dependencies. N-API notices are pinned to the crate Git revisions. There is no added attribution on generated PNG, SVG or HTML. Original Monoprint code is Apache-2.0; bundled material retains its own licenses. See [third-party notices](../THIRD_PARTY_NOTICES.md).
