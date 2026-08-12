# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Aidoku Chinese manga source collection. Each source is its **own crate** in its own directory under `sources/{source-id}/` (mirroring [Aidoku-Community/sources](https://github.com/Aidoku-Community/sources)) so different sources don't mix. Currently hosts:

- **`zh.hipmh`** — 嬉皮漫画 (https://m.hipmh.com). Based on the `zh.happymh` source from Aidoku-Community, adapted for hipmh.com; uses the redesigned site's `/v1/*` JSON APIs.
- **`zh.bilimanga`** — 嗶哩漫畫 (https://www.bilimanga.net). Copied from Aidoku-Community; pure HTML scraper.

Each source ships its own `package.aix` (a zip with `Payload/main.wasm` + `res/*`) and pins its **own aidoku-rs git rev** via its committed `Cargo.lock` — the two crates currently use different revs (`zh.hipmh` → `1a6bb691`, `zh.bilimanga` → `b0818704`), so they must stay separate crates and never be merged into one.

## Build

Requires **Rust nightly** (edition 2024) with the `wasm32-unknown-unknown` target (pinned in each crate's `.cargo/config.toml`).

```bash
# Install wasm target
rustup target add wasm32-unknown-unknown

# Build all sources (bash)
./build.sh

# Build all sources (PowerShell)
./build.ps1
```

Both scripts iterate `sources/*/` — for each source: `(cd "$src" && cargo +nightly build --release)` then `python pack.py "$src"`. Output is one `package.aix` **per source** (`sources/zh.hipmh/package.aix`, `sources/zh.bilimanga/package.aix`).

`pack.py <src_dir>` globs `<src_dir>/target/wasm32-unknown-unknown/release/*.wasm` (one per crate), zips it with `<src_dir>/res/*` into `<src_dir>/package.aix`.

**Icon gotcha:** an installed source's icon is loaded from `Payload/icon.png` inside the `.aix` (AidokuRunner hardcodes `imageUrl = <source_dir>/icon.png`), NOT from the source-list `iconURL` (that one only shows pre-install in the source list). If `res/icon.png` is missing, the .aix installs fine but the icon shows Aidoku's placeholder. Keep `res/icon.png` in each source's package.

**Packaging gotcha:** the zip is built by `pack.py` (python `zipfile`, forward-slash arcnames), not PowerShell `Compress-Archive`. On Windows, `Compress-Archive` writes entry names like `Payload\main.wasm` (backslash); iOS/macOS ZIPFoundation then extracts one literal `Payload\main.wasm` file instead of a `Payload/` folder, so Aidoku fails to load the source ("解压/解析失败"). The zip MUST contain forward-slash `Payload/*` entries. `pack.py` asserts this on output.

**Cargo.lock:** each source's `Cargo.lock` is **committed** (`.gitignore` has `!sources/*/Cargo.lock`) to pin its aidoku-rs rev — without the pin, a fresh `cargo build` resolves aidoku-rs to master HEAD, which may not compile the source. Bump a lock deliberately (e.g. `cd sources/<id> && cargo update -p aidoku`).

`aidoku-test` is declared in `[dev-dependencies]`, but **no tests exist yet** — don't assume a test suite that isn't there.

## Architecture — zh.hipmh (API + HTML hybrid)

Implements the Aidoku trait-based API (Aidoku 0.7+), registered via `register_source!` at the bottom of `sources/zh.hipmh/src/lib.rs`. The `Hipmh` struct implements `Source` (search, manga update, page list), `ImageRequestProvider` (adds `Referer` header), `DeepLinkHandler` (URL parsing), and `ListingProvider` (catalog via the `/v1/mangas` JSON API).

```
sources/zh.hipmh/src/
├── lib.rs                    # Source traits, search/browse/listing via /v1 APIs, deep links, BASE_URL
├── html/mod.rs               # HTML scraper: /works/ detail parsing
└── json/
    ├── chapter_list/mod.rs   # Chapter list via /v1/manga/chapters API
    └── page_list.rs          # Page images via /v2/chapter API + char-transform decoder
```

### hipmh API Endpoints

| Purpose | Method | Path | Notes |
|---------|--------|------|-------|
| Catalog listings | GET | `https://hipapi1.s3file.top/v1/mangas?sort={s}&page={p}&per_page=18` | Working. `sort` ∈ popular/weekly/latest, optional `status` ∈ completed/ongoing. Response `{code:200, data:{items, page, total_pages}}`; item `mid` is the `/works/` slug, cover fields need `https://cover.s3imgs.top` prefix |
| Manga Detail | GET | `https://m.hipmh.com/works/{base64}` | Working (id-only base64, no slug needed). Parsed in `sources/zh.hipmh/src/html/mod.rs` via `[data-manga-title]`/`[data-cover-url]`, `#d-info-content p`, `a[href^="/author/"]`, `a[href^="/genre/"]`, status via `/ongoing`/`/completed` link |
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
| Image decode | Pure char transform (no crypto) | **Working** (`decode_images` in `sources/zh.hipmh/src/json/page_list.rs`, verified byte-for-byte against `reader.hipmh.top/assets/runtime/chapter-decoder.js`). Strip prefix `qM9` + suffix `Z7` → inner. `total = len(inner)-5`; `k_len = total/3`, `a_len = (total-k_len)/2`, `b_len = total-k_len-a_len`. Layout `A + "Vx" + B + "pL0" + K`; recombine as `K + A + B`. Split into 7-char chunks, reverse every odd chunk (idx%2). Substitute chars FROM = `_-9876543210...Z` → TO = `ABC...-_` (scrambled→standard url-safe base64 alphabet). Url-safe base64 (no padding) → UTF-8 JSON array of relative `/i/...` paths. Image URL = `https://hip-tx-{line}.s3imgs.top{path}`, `Referer: https://m.hipmh.com/` |

**Current status:** catalog, manga detail, chapter list, search, and **page images** all work. The image decoder is fully reverse-engineered and implemented in Rust (`sources/zh.hipmh/src/json/page_list.rs`).

### hipmh Key Logic

**Search + browse (`lib.rs` `get_search_manga_list`)** — A text filter is treated as a search query (via `/v1/search?q=...&page=&page_size=20`). With no query, browse filters map to `/v1/mangas` — only the 状态 filter works (`status=ongoing|completed`); 类型/地区 need numeric IDs the API doesn't expose, so they're skipped.

**Catalog listings (`lib.rs` `ListingProvider`)** — All five listings (人气榜/本周热门/最新上架/完结/连载) call `/v1/mangas` with `sort`/`status` params. `parse_manga_list` (shared with search) handles both `data.items` (listings, item `mid`) and `data.data` (search, item `id`) shapes. Manga keys come from the `/works/` slug via `works_slug_to_key` (base64-decode the leading segment → `m:23475`). Covers are relative and get prefixed with `https://cover.s3imgs.top`. `has_next_page = page < total_pages`.

**Chapter list (`sources/zh.hipmh/src/json/chapter_list/mod.rs`)** — Calls `/v1/manga/chapters` with `mid` (numeric id, `m:` prefix stripped from the key), `order=desc` (newest first). **The API caps `per_page` at 50** (hard cap — tested up to 1000; no other param raises it), so a long manga spans many pages. Page 1 is fetched first to learn `total_pages`, then the remaining pages are fetched **concurrently via `Request::send_all`** (results come back in request order, so newest-first is preserved). `send_all` is safe on iOS: the AidokuRunner runtime (rev pinned in Package.resolved) registers/implement it; the app's `Shared/Wasm/Imports/WasmNet.swift` (only `send`) is dead code not instantiated. `RateLimit` defaults to disabled (`permits=0`), so no throttling. Chapter `key` is the item's `hid`; `url` is `/chapter/go?hid={hid}&m={mid}`.

**Chapter dates (v7):** each chapter's `date_uploaded` is parsed from the item's `updated_at` (fallback `created_at`), an ISO-8601 UTC string (`2026-08-07T10:31:40.275232Z`). `parse_iso8601_to_epoch` slices the fixed fields and converts via `days_from_civil` (Howard Hinnant's proleptic-Gregorian algorithm) — no std/chrono available in no_std. aidoku-rs at the pinned rev has no date helper, so this is hand-rolled. Aidoku renders the date in the chapter list automatically once `date_uploaded` is set.

**Page images (`sources/zh.hipmh/src/json/page_list.rs`)** — `get_pages` derives `api_hid` from the chapter `key` (the `/v1/manga/chapters` `hid`): `rsplit_once('-')` → seg1 (base64 of `m:{mid}-c:{cid}`, padding may be stripped) and seg2; `api_hid = base64("c:{cid}")` with `=` trimmed + `-` + seg2. Then `GET /v2/chapter?hid={api_hid}` with `Origin`/`Referer: https://reader.hipmh.top` (no auth needed). The response `data.images` is decoded by `decode_images` — a pure character transform (no crypto, see the table above) producing a JSON array of relative `/i/...` paths. Page URLs are built as `https://hip-tx-{data.line}.s3imgs.top{path}`; image requests use `Referer: https://m.hipmh.com/` via `ImageRequestProvider`.

If image decoding breaks, re-verify against the live `reader.hipmh.top/assets/runtime/chapter-decoder.js` (the probe files `.probe-*.js`/`.probe-*.txt` live in `sources/zh.hipmh/`, git-ignored, reproduce the transform). The `TRANSLATE` substitution table and the `qM9`/`Vx`/`pL0`/`Z7` markers are the invariants to check.

**HTML parsing (`sources/zh.hipmh/src/html/mod.rs`)** — `update_details` parses the `/works/{base64_id}` page (id-only URL works, no slug needed). Uses attribute selectors (SwiftSoup, full CSS): `[data-manga-title]` and `[data-cover-url]` on the app container, `#d-info-content p` (description), `a[href^="/author/"]` (authors), `a[href^="/genre/"]` (tags). Status: presence of `a[href='/completed']`/`a[href='/ongoing']` → `MangaStatus`. `Viewer::Webtoon` is set for all manga.

**Base64 padding gotcha (v5):** the `/works/{id}` base64 in site URLs is **padding-stripped** (`bToyMzQ3NQ`, never `bToyMzQ3NQ==`). base64 0.22's `STANDARD` engine requires canonical `=` padding on **decode** too (RequireCanonical), so both directions need padding handling:
- `decode_work_id` (slug first segment → key) MUST re-pad before decoding — `trim_end_matches('=')` then pad to len%4==0. Otherwise an unpadded ID like `bToyMzQ3NQ` fails to decode and the raw string leaks through as the manga key, which then gets double-encoded into a wrong `/works/` URL → 404, and `update_details` on the 404 page clears `manga.cover` (no `[data-cover-url]`). Only manga whose id yields no padding (e.g. `m:9711` → `bTo5NzEx`, 8 chars) worked.
- `key_to_works_id` (key → URL segment) MUST `.trim_end_matches('=')` off `STANDARD.encode`'s output (padded paths 404 on the Nuxt router).
- Invariants are covered by the `#[aidoku_test]` round-trip test in `sources/zh.hipmh/src/lib.rs` (run via `cargo +nightly test` — needs `aidoku-test-runner` installed and the `[target.wasm32-unknown-unknown] runner` line in `sources/zh.hipmh/.cargo/config.toml`).

**Deep Link Handling** — `/works/{base64_id}--{slug}` (base64-decodes the ID → `m:{numeric_id}`, used as manga key); `/manga/{id}` and `/manga/{id}/{chapter_id}` direct internal links.

## Architecture — zh.bilimanga (pure HTML scraper)

Copied from Aidoku-Community (2026-08), `aes` dependency removed (never used). Source id `zh.bilimanga` (嗶哩漫畫), `BASE_URL = "https://www.bilimanga.net"`. Deps: `aidoku {json}` + `regex` (chapter-number extraction). **All requests** send `User-Agent: <mobile UA>` (a desktop Chrome UA makes the reader serve a "请使用手机浏览器" page with no images), `Origin: <BASE_URL>`, `Accept-Language: zh-CN,zh;q=0.9`, `Cookie: night=0`.

```
sources/zh.bilimanga/src/
├── lib.rs        # Source traits + deep links (/detail/{id}.html, /read/{manga}/{chapter}.html)
├── html/mod.rs   # Document parsing: manga page, chapter list, page images
└── net/mod.rs    # Url enum: builds filter/search/author/detail/catalog/chapter URLs
```

- **Search** — `/search.html?searchkey={q}` (page 1) or `/search/{q}_{page}.html`; a non-empty text filter becomes a search, an `author` filter hits `/author/{value}.html`.
- **Browse/filters** — default is the 12-segment `/filter/{order}_{tagid}_{isfull}_{anime}_{rgroupid}_{sortid}_{update}_{quality}_{page}_0_0_0.html` (the upstream 10-segment format `..._0.html` 404s since the redesign). Positions verified: page at segment 9, sortid at 6, isfull at 3.
  - **作品分类 (v4)**: builds the site's **named category URL** `/filter/{slug}/{page}.html` instead of the numeric `sortid` segment — the numeric sortid only covers a subset (科幻未来 sortid=8 is flaky, 奇异幻想 sortid=9 404s). Slug map `CATEGORY_SLUGS` in `net/mod.rs`: 奇幻冒险→FantasyAdventure, 战斗热血→Action, 悬疑惊悚→SuspenseHorror, 校园青春→SchoolLife, 爱情浪漫→Romance, 职场都市→Workplace, 历史文化→Historical, 科幻未来→**ScienceFiction** (the site's own `Sci-Fi` slug 404s), 奇异幻想→Supernatural, 治愈温馨→Healing, 末日生存→Survival, 其他分类→Other.
  - **作品主题 (v4)**: changed from multi-select to **single-select** (`tagid = value`) — multi-value tagids (`1-2`) 404 on the site; only single works. `filters.json` gained a "不限" (id 0) option.
- **Manga detail** — `/detail/{id}.html`; parse `.book-cover` (cover), `h1.book-title`, `.authorname,.illname` (authors), `.book-summary>content` (description), `.tag-small-group>.tag-small>a` (tags), `.book-layout-inline` (first `|`-field → status 連載=Ongoing/完結=Completed). `Viewer` from tags: 大陸/韓國 → Webtoon, 日本 → RightToLeft, else LeftToRight. An `.aui-ver-form` block means removed content — its text goes into `manga.description`.
- **Chapter list** — `/read/{id}/catalog`; group by `.catalog-volume` (volume number from the `<h3>`), chapters via `.chapter-li-a`. Some volumes' links are `javascript:` — those fetch the volume's own page (`{BASE_URL}{vol_href}` with `Origin` header) to get real links. `volume_thumbnail` (`.volume-cover-img img[data-src]`) is set as each chapter's `thumbnail`. Chapters are `reverse()`d (newest first).
- **Pages** — `/read/{manga_id}/{chapter_id}.html`; images are `#acontentz>img[data-src]`. The images themselves live on `i.motiezw.com`, which is Cloudflare-gated at the IP level (my datacenter IP gets 403; a residential IP / the Aidoku app should pass).
- **Search (v4 note, site-side blocked)** — the site's `/search.html` (form field `searchkey`, POST on the site) returns an empty body (HTTP 200, 0 bytes) to all non-browser HTTP clients (GET/POST, `searchkey`/`keyword`, with session cookies, full browser headers + client hints, and the source's own headers) — verified 2026-08-12. Search results are loaded client-side behind Cloudflare; there is no server-rendered results URL (sitemap has none) and no discoverable JSON API. The source keeps the original `/search.html?searchkey={q}` / `/search/{q}_{page}.html` URLs (matches the site form), so it may work in the Aidoku app's real HTTP stack where curl is gated — but no source-side change can guarantee it. If the user reports search still empty in-app, the site has gated it and search is effectively unavailable for this source.
- **Images** — `ImageRequestProvider` sets `Referer: <BASE_URL>`.
- **has_next_page** (manga list) — from `#pagelink`: `strong` (current) vs `.last`, else `.next` href != `#`.

## Deploy

Each built `sources/{id}/package.aix` is versioned into `public/` (`public/sources/{id}-v{n}.aix` + `public/icons/{id}-v{n}.png`) and listed in `public/index.json`/`public/index.min.json` (the Aidoku source list). On version bump of a source: change `version` in the manifest AND that source's `res/source.json` `info.version`, rebuild, and rename its `public/` files (a fresh filename also sidesteps raw CDN caching). The `gh-pages` branch carries the same `public/` files the app needs and is generated from the `master` `public/` directory. Repo: `doyayaa/aidoku-source-zh`.

**Distribution entry point is `raw.githubusercontent.com`**, not GitHub Pages:
- Source list URL: `https://raw.githubusercontent.com/doyayaa/aidoku-source-zh/gh-pages/index.min.json`
- `iconURL` and `downloadURL` MUST be **relative** paths (`icons/zh.hipmh-v8.png`, `sources/zh.bilimanga-v4.aix`) resolved against the source-list URL. **Absolute URLs break import in Aidoku** (verified by user on 2026-08-12) — do not use absolute paths for these fields.
- GitHub Pages is NOT used: the repo's custom domain `doyayaa.online` is deprecated, and `doyayaa.github.io/...` still 301s to it (can't be cleared via API; needs web UI Settings → Pages → Custom domain → Clear if ever re-enabled).

**The manifest MUST be the new `{"name": ..., "sources": [...]}` format** (fields `iconURL`/`downloadURL`/`languages`/`contentRating`/`baseURL`/`minAppVersion`). The old flat-array format (`file`/`icon`/`lang`/`nsfw`) causes Aidoku to label the whole source list "旧版图源" (legacy). Keep `public/` and the `gh-pages` branch in sync. The hipmh `iconURL` is an absolute remote URL (`https://m.hipmh.com/assets/logo.C1THqItK.png`, the site's logo); `public/icons/zh.hipmh-v1.png` is a legacy local copy no longer referenced.

## Key Dependencies

- `aidoku` (git: Aidoku/aidoku-rs) — core framework; hipmh uses `helpers` + `json` features, bilimanga uses `json`
- `serde`/`serde_json` — JSON parsing (hipmh)
- `base64` — hipmh work-ID encoding/decoding + url-safe decode of image payloads
- `regex` — bilimanga chapter-number extraction

## Differences from zh.happymh (hipmh only)

- `BASE_URL` changed to hipmh.com
- Deep link handler added support for `/works/` URL pattern (hipmh's Nuxt frontend format)
- Catalog/detail/chapters/search all use the redesigned site's `/v1/*` APIs instead of the old `/v2.0/apis/*` + `/rank/*` HTML
