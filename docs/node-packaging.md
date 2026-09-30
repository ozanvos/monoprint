# Node package builds

From `bindings/node`, with Rust 1.97+, a C compiler, Node.js 22+, and the checked-in Cargo/npm locks:

```sh
npm ci
npm run build
npm test
npm run test:types
npm run test:package
```

The last command packs the host binary and installs the tarballs into a clean, relocated consumer. It tests ESM, CommonJS, TypeScript resolution and a server bundle with install scripts disabled. These local-only artifacts cannot be published by the release script.

The `npm-packages.yml` workflow builds and tests each target, collects the exact binaries and their source/checksum receipts, then uploads a complete `npm-release` artifact. No workflow step publishes anything. To publish:

1. Set the final npm package name and optional repository metadata in `package.json`. The default name is `monoprint`; ownership/availability must be checked for it and all five platform names before publication. Keep the npm and `monoprint-node` Cargo versions equal, update the npm lock, and commit the intended source.
2. Run the workflow and download its `npm-release` artifact into a fresh directory. Every declared target must pass its native runtime tests. The artifact contains `manifest.json` and `tarballs/`.
3. From the matching source checkout, verify the artifact without contacting npm:

   ```sh
   npm run publish:packages -- --from /path/to/npm-release --dry-run
   ```

4. Public publication is disabled in `release/config.json`. After an approved
   launch, configure account ownership and trusted publishing for all six names,
   enable `publicPublishing` in `release/config.json`, and use the reviewed manual release workflow.
   An explicit local invocation additionally requires:

   ```sh
   MONOPRINT_PUBLIC_PUBLISH=approved npm run publish:packages -- --from /path/to/npm-release --publish
   ```

Platform packages publish first and the main package last. The script uses `--ignore-scripts`; neither packing nor a dry run can trigger hidden publication. Versions on npm are immutable and multi-package publication is not atomic. If publication stops partway, inventory the published versions, compare their archive integrity to the saved manifest, and publish only the remaining unchanged tarballs with `npm publish /path/to/file.tgz --access public --ignore-scripts`. Do not rebuild different binaries under the same version. Optional `--tag next` selects a prerelease dist-tag; prerelease versions default to `next`.

Without `--publish`, the publisher only validates, even if `--dry-run` is omitted.
Receipts bind native files and tarballs to one source commit and input hash;
mixed revisions, dirty release builds and altered packed metadata are rejected.
See [release packaging](release.md) for workflow configuration and npm OIDC setup.

To assemble already-built release artifacts yourself, use `npm run package -- --artifacts /path/to/native-artifacts --out /fresh/output`. It refuses missing, duplicate, stale, wrong-version, or altered native artifacts. `--local` creates an explicitly non-release subset for local testing. Output paths must be fresh; packaging never deletes an existing caller-selected directory.

Every tarball includes project/third-party notices, font licenses, MPL grammar sources, and unchanged source archives for Giallo and MPL-covered Rust dependencies. N-API notices are pinned to the crate Git revisions. There is no added attribution on generated PNG, SVG or HTML. Original Monoprint code is Apache-2.0; bundled material retains its own licenses. See [third-party notices](../THIRD_PARTY_NOTICES.md).
