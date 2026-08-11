# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Aidoku manga source extension for **嬉皮漫画** (https://m.hipmh.com). Based on the `zh.happymh` source from [Aidoku-Community/sources](https://github.com/Aidoku-Community/sources), adapted for the hipmh.com domain. Distributed as `zh.hipmh` via GitHub Pages.

## Build

Requires **Rust nightly** (edition 2024) with the `wasm32-unknown-unknown` target (pinned in `.cargo/config.toml`).

```bash
# Install wasm target
rustup target add wasm32-unknown-unknown

# Build (bash)
./build.sh

# Build (PowerShell)
./build.ps1
```

Output: `package.aix` — a zip containing `main.wasm` and `res/*`. This is loaded by the Aidoku app. `res/source.json` declares the source metadata (`id: zh.hipmh`, version, listings); `res/filters.json` drives the browse filter UI.

`aidoku-test` is declared in `[dev-dependencies]`, but **no tests exist yet** — don't assume a test suite that isn't there.

## Architecture

Implements the Aidoku trait-based API (Aidoku 0.7+), registered via `register_source!` at the bottom of `src/lib.rs`. The `Hipmh` struct implements `Source` (search, manga update, page list), `ImageRequestProvider` (adds `Referer` header), `DeepLinkHandler` (URL parsing), and `ListingProvider` (catalog via the `/v1/mangas` JSON API).

```
src/
├── lib.rs                    # Source traits, search/browse/listing via /v1 APIs, deep links, BASE_URL
├── html/mod.rs               # HTML scraper: /works/ detail parsing
└── json/
    ├── chapter_list/mod.rs   # Chapter list via /v1/manga/chapters API
    └── page_list.rs          # Page images via /v2/chapter API + char-transform decoder
```

## API Endpoints

| Purpose | Method | Path | Notes |
|---------|--------|------|-------|
| Catalog listings | GET | `https://hipapi1.s3file.top/v1/mangas?sort={s}&page={p}&per_page=18` | Working. `sort` ∈ popular/weekly/latest, optional `status` ∈ completed/ongoing. Response `{code:200, data:{items, page, total_pages}}`; item `mid` is the `/works/` slug, cover fields need `https://cover.s3imgs.top` prefix |
| Manga Detail | GET | `https://m.hipmh.com/works/{base64}` | Working (id-only base64, no slug needed). Parsed in `html/mod.rs` via `[data-manga-title]`/`[data-cover-url]`, `#d-info-content p`, `a[href^="/author/"]`, `a[href^="/genre/"]`, status via `/ongoing`/`/completed` link |
| Chapter List | GET | `https://hipapi1.s3file.top/v1/manga/chapters?mid={numeric_id}&page={p}&per_page=50&order=desc` | Working. Item `hid` is the chapter key, `title`, `chapter_number`. API caps `per_page` at 50 |
| Search | GET | `https://hipapi1.s3file.top/v1/search?q={query}&page={p}&page_size=20` | Working. Response `{code:200, data:{data:[items], total, page, total_pages}}`; item `id` is the full `/works/` slug |
| Browse (filters) | GET | `/v1/mangas?...` | Only the 状态 filter maps (`status=ongoing\|completed`); 类型/地区 need numeric IDs not exposed by the API, so they're skipped |
| Page Images | GET | `https://hipapi1.s3file.top/v2/chapter?hid={api_hid}` | Working. Needs `Origin`/`Referer: https://reader.hipmh.top`. Response `data.images` is an encrypted string, `data.line` selects the image CDN. `api_hid` derived from the chapter `hid` (see below) |
| ~~Rankings~~ | GET | `~~/rank/{type}?page={p}~~` | **Broken** — old rank pages gone |

**Site redesign note (2026-08):** m.hipmh.com was rebuilt (Astro, Traditional Chinese). The old `/v2.0/apis/*` endpoints, `/rank/*` pages, and `/manga/{id}` detail pages all return empty. The current working endpoints:

| Purpose | Endpoint | Notes |
|---------|----------|-------|
| Catalog listings | `https://hipapi1.s3file.top/v1/mangas?sort={s}&status={st}&page={p}&per_page={n}` | `sort` ∈ popular/weekly/latest, `status` ∈ completed/ongoing. Item `mid` = `/works/` slug |
| Manga detail | `GET /works/{base64_id}` (id-only works) | Parse JSON-LD (`<script type="application/ld+json">`): `name` (title), `description`, `image`, `author.name` (comma-sep). Authors also `a[href^="/author/"]`, tags `a[href^="/genre/"]`, status via `/ongoing`(連載中) or `/completed`(完結) link |
| Chapter list | `https://hipapi1.s3file.top/v1/manga/chapters?mid={numeric_id}&page={p}&per_page={n}&order={desc\|asc}` | Item `hid` = chapter key (base64 "m:{mid}-c:{cid}--{mid}:{num}.00"), `title`, `chapter_number` |
| Reading | `GET /v2/chapter?hid={api_hid}` on hipapi1 | `api_hid` derives from frontend `hid`: first segment `bToyMzQ3NS1jOjc4MzI5` decodes to `m:23475-c:78329`, api_hid = base64(`c:78329`, padding stripped) + `-` + second segment → `Yzo3ODMyOQ-MjM0NzU6ODI1LjAw`. Response `data.images` is an encrypted string, `data.line` = image CDN line number |
| Image decode | Pure char transform (no crypto) | **Working** (`decode_images` in `src/json/page_list.rs`, verified byte-for-byte against `reader.hipmh.top/assets/runtime/chapter-decoder.js`). Strip prefix `qM9` + suffix `Z7` → inner. `total = len(inner)-5`; `k_len = total/3`, `a_len = (total-k_len)/2`, `b_len = total-k_len-a_len`. Layout `A + "Vx" + B + "pL0" + K`; recombine as `K + A + B`. Split into 7-char chunks, reverse every odd chunk (idx%2). Substitute chars FROM = `_-9876543210...Z` → TO = `ABC...-_` (scrambled→standard url-safe base64 alphabet). Url-safe base64 (no padding) → UTF-8 JSON array of relative `/i/...` paths. Image URL = `https://hip-tx-{line}.s3imgs.top{path}`, `Referer: https://m.hipmh.com/` |

**Current status:** catalog, manga detail, chapter list, search, and **page images** all work. The image decoder is fully reverse-engineered and implemented in Rust (`src/json/page_list.rs`).

## Key Logic (across files)

### Search + browse (lib.rs `get_search_manga_list`)
A text filter is treated as a search query (via `/v1/search?q=...&page=&page_size=20`). With no query, browse filters map to `/v1/mangas` — only the 状态 filter works (`status=ongoing|completed`); 类型/地区 need numeric IDs the API doesn't expose, so they're skipped.

### Catalog listings (lib.rs `ListingProvider`)
All five listings (人气榜/本周热门/最新上架/完结/连载) call `https://hipapi1.s3file.top/v1/mangas` with `sort`/`status` params. `parse_manga_list` (shared with search) handles both `data.items` (listings, item `mid`) and `data.data` (search, item `id`) shapes. Manga keys come from the `/works/` slug via `works_slug_to_key` (base64-decode the leading segment → `m:23475`). Covers are relative and get prefixed with `https://cover.s3imgs.top`. `has_next_page = page < total_pages`.

### Chapter list (json/chapter_list/mod.rs)
Calls `https://hipapi1.s3file.top/v1/manga/chapters` with `mid` (numeric id, `m:` prefix stripped from the key), `order=desc` (newest first), paginating until `page >= total_pages`. The API caps `per_page` at 50. Chapter `key` is the item's `hid`; `url` is `/chapter/go?hid={hid}&m={mid}`.

### Page images (json/page_list.rs)
`get_pages` derives `api_hid` from the chapter `key` (the `/v1/manga/chapters` `hid`): `rsplit_once('-')` → seg1 (base64 of `m:{mid}-c:{cid}`, padding may be stripped) and seg2; `api_hid = base64("c:{cid}")` with `=` trimmed + `-` + seg2. Then `GET https://hipapi1.s3file.top/v2/chapter?hid={api_hid}` with `Origin`/`Referer: https://reader.hipmh.top` (no auth needed). The response `data.images` is decoded by `decode_images` — a pure character transform (no crypto, see the table above) producing a JSON array of relative `/i/...` paths. Page URLs are built as `https://hip-tx-{data.line}.s3imgs.top{path}`; image requests use `Referer: https://m.hipmh.com/` via `ImageRequestProvider`.

If image decoding breaks, re-verify against the live `reader.hipmh.top/assets/runtime/chapter-decoder.js` (the probe files `.probe-*.js`/`.probe-*.txt` in the repo root, git-ignored, reproduce the transform). The `TRANSLATE` substitution table and the `qM9`/`Vx`/`pL0`/`Z7` markers are the invariants to check.

### HTML parsing (html/mod.rs)
`update_details` parses the `/works/{base64_id}` page (id-only URL works, no slug needed). Uses attribute selectors (SwiftSoup, full CSS): `[data-manga-title]` and `[data-cover-url]` on the app container, `#d-info-content p` (description), `a[href^="/author/"]` (authors), `a[href^="/genre/"]` (tags). Status: presence of `a[href='/completed']`/`a[href='/ongoing']` → `MangaStatus`. `Viewer::Webtoon` is set for all manga.

### Deep Link Handling
- `/works/{base64_id}--{slug}` — base64-decodes the ID (decodes to `m:{numeric_id}`), used as the manga key
- `/manga/{id}` and `/manga/{id}/{chapter_id}` — direct internal links

## Deploy

Built `package.aix` is placed at `public/sources/zh.hipmh-v1.aix` alongside `public/index.json`/`public/index.min.json` (the Aidoku source list). The `gh-pages` branch carries the same files the app needs and is generated from the `master` `public/` directory. Repo: `doyayaa/aidoku-source-zh-hipmh`.

**Distribution entry point is `raw.githubusercontent.com`**, not GitHub Pages:
- Source list URL: `https://raw.githubusercontent.com/doyayaa/aidoku-source-zh-hipmh/gh-pages/index.min.json`
- `downloadURL` is an **absolute** URL (`https://raw.githubusercontent.com/doyayaa/aidoku-source-zh-hipmh/gh-pages/sources/zh.hipmh-v2.aix`), so source download works regardless of how the source list URL is entered; the `.aix` filename carries the version (`zh.hipmh-v{n}.aix`, mirrors the manifest `version`). `iconURL` is absolute too.
- On version bump: change `version` in the manifest AND `res/source.json` `info.version`, rebuild, and rename `public/sources/zh.hipmh-v{n}.aix` (a fresh filename also sidesteps raw CDN caching).
- GitHub Pages is NOT used: the repo's custom domain `doyayaa.online` is deprecated, and `doyayaa.github.io/...` still 301s to it (can't be cleared via API; needs web UI Settings → Pages → Custom domain → Clear if ever re-enabled).

**The manifest MUST be the new `{"name": ..., "sources": [...]}` format** (fields `iconURL`/`downloadURL`/`languages`/`contentRating`/`baseURL`/`minAppVersion`). The old flat-array format (`file`/`icon`/`lang`/`nsfw`) causes Aidoku to label the whole source list "旧版图源" (legacy). Keep `public/` and the `gh-pages` branch in sync. `iconURL` is an absolute remote URL (`https://m.hipmh.com/assets/logo.C1THqItK.png`, the site's logo); `public/icons/zh.hipmh-v1.png` is a legacy local copy no longer referenced.

## Key Dependencies

- `aidoku` (git: Aidoku/aidoku-rs) — core framework with `helpers` + `json` features
- `serde`/`serde_json` — JSON parsing
- `base64` — work-ID encoding/decoding + url-safe decode of image payloads

## Differences from zh.happymh

- `BASE_URL` changed to hipmh.com
- Deep link handler added support for `/works/` URL pattern (hipmh's Nuxt frontend format)
- Catalog/detail/chapters/search all use the redesigned site's `/v1/*` APIs instead of the old `/v2.0/apis/*` + `/rank/*` HTML
