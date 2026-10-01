import { defineConfig } from 'astro/config';
import release from '../release/config.json' with { type: 'json' };

export default defineConfig({
  output: 'static',
  site: release.homepage ?? undefined,
  publicDir: process.env.MONOPRINT_SITE_PUBLIC_DIR ?? './.generated/public',
  devToolbar: { enabled: false },
});
