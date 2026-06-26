# Site

The public site at [jazyk.org](https://jazyk.org). Plain static HTML with
[Tailwind CSS](https://tailwindcss.com) loaded from its CDN, so there is no
build step. Specified under [`../../docs/site`](../../docs/site).

## Pages

| File | Route |
| --- | --- |
| `index.html` | `/` |
| `compilation/index.html` | `/compilation` |
| `artifact/index.html` | `/artifact` |
| `favicon.svg` | `/favicon.svg` (copied from `docs/logo.svg`) |
| `CNAME` | custom domain for GitHub Pages |

## Preview locally

```sh
cd bootstrap/site
python3 -m http.server 8000   # then open http://localhost:8000
```

## Deploy

Pushed to `master` and published to GitHub Pages by
[`.github/workflows/site.yml`](../../.github/workflows/site.yml). No manual step.
