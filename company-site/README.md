# Seven East company page

Static company homepage for https://7ea.st. Initial HTML and CSS designed by
Claude Code using `claude-opus-5-5` on 2026-10-23 at the owner's request.
No build step, client JavaScript, external fonts, or analytics.

From the repository root:

```sh
python3 -m http.server 4181 --bind 127.0.0.1 --directory company-site/public
website/node_modules/.bin/wrangler deploy --config company-site/wrangler.jsonc --dry-run
website/node_modules/.bin/wrangler deploy --config company-site/wrangler.jsonc
```

Deployment targets only the `seven-east-site` Worker and apex `7ea.st` domain.
The Monoprint website and email DNS records are independent.
