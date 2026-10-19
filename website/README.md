# Monoprint website

Astro static site at [monoprint.7ea.st](https://monoprint.7ea.st), hosted with
Cloudflare Workers Static Assets. Fonts and images are served locally.
The hero uses a native macOS screenshot from the App Store captures. Astro generates
responsive image sizes; the export examples are rendered by the Monoprint CLI
from `assets/examples/`. Their themes, highlights and captions are defined in
`src/data/showcase.mjs`. The theme picker samples and their CLI commands come
from `src/data/demo.mjs`.

## Development

Requires Node.js 22.12+ and a built Monoprint CLI:

```sh
cargo build --locked --release
npm ci --prefix website
npm run dev --prefix website
npm run check --prefix website
npm run build --prefix website
npm run preview --prefix website
```

Set `MONOPRINT_CLI` to an absolute path to use another executable. Asset
preparation generates sample images, copies fonts and copies support/privacy
pages from `release/app-store/pages/`. Generated inputs are in `.generated/`;
build output is in `dist/`.

For browser checks, install the root npm dependencies and Playwright Chromium,
or set `CHROME_PATH` to an installed Chrome executable:

```sh
npm ci --ignore-scripts
npx playwright install chromium
node scripts/test-website.mjs --from website/dist --out output/playwright/website
```

## Deployment

`wrangler.jsonc` configures the `monoprint-site` Worker and its custom domain.
Deployments upload `dist/`. Nothing deploys on push. `websitePublishing` in
`release/config.json` controls website publication; `publicPublishing`
separately controls app/npm releases and source links. Download links stay
inactive until a public release is available.

After checking and building an approved update:

```sh
npm run check:cloudflare --prefix website
MONOPRINT_WEBSITE_PUBLISH=approved npm run deploy --prefix website
```

`check:cloudflare` is a deployment dry run. `preview:cloudflare` serves the site
locally with Cloudflare's asset headers and 404 handling. Set `CLOUDFLARE_ACCOUNT_ID` in your local environment to select the deployment
account. Wrangler uses your authenticated credentials; keep them outside Git. See the
[release guide](../docs/release.md) for publication controls.
