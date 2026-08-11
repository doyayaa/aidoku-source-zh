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

Implements the Aidoku trait-based API (Aidoku 0.7+), registered via `register_source!` at the bottom of `src/lib.rs`. The `Hipmh` struct implements `Source` (search, manga update, page list), `ImageRequestProvider` (adds `Referer` header), `DeepLinkHandler` (URL parsing), and `ListingProvider` (rankings).

```
src/
├── lib.rs                    # Source traits, URL dispatch, deep links, BASE_URL
├── net/mod.rs                # URL construction, filter parsing, GENRE_OPTIONS/GENRE_IDS arrays
├── html/mod.rs               # HTML scraper: manga detail + ranking page selectors
└── json/
    ├── chapter_list/mod.rs   # Chapter list (paginated asc, then reversed)
    └── page_list.rs          # Page images + scan decryption + anti-bot headers
```

## API Endpoints

| Purpose | Method | Path | Notes |
|---------|--------|------|-------|
| Browse/Filter | GET | `/apis/c/index?&order=last_date&genre={g}&area={a}&audience={au}&series_status={s}&pn={p}` | Returns JSON |
| Search | POST | `/v2.0/apis/manga/ssearch` | Body: `searchkey={q}&v=v2.13&page={p}`, referer `/search` |
| Manga Detail | GET | `/manga/{id}` | Returns HTML, parsed in `html/mod.rs` |
| Chapter List | GET | `/v2.0/apis/manga/chapterByPage?code={id}&page={p}&lang=cn&order=asc` | Paginated, `isEnd` field signals end |
| Page Images | GET | `/v2.0/apis/manga/reading?code={mid}&cid={chid}&v=v4.300101&_t={ts}` | Encrypted scans |
| Rankings | GET | `/rank/{type}?page={p}` | Returns HTML, parsed in `html/mod.rs` |

## Key Logic (across files)

### Search response normalization (lib.rs `get_search_manga_list`)
The search endpoint has returned several response shapes over time. The code defensively checks three locations — `data.items`, top-level `items`, `payload.items` — and each may be a real array **or a string containing JSON**. Don't "simplify" this into a single shape; the site varies.

### Filter → URL mapping (net/mod.rs)
`res/filters.json` defines the select filters (`地区`, `受众`, `状态`, `类型`). `Url::from_query_or_filters` matches these by their Chinese `id`. The genre select has two id cases:
- `"类型"` (uppercase, from filters.json) → value is used as the raw pinyin slug.
- `"genre"` (lowercase) → the Chinese option name is mapped to its pinyin slug via the parallel `GENRE_OPTIONS`/`GENRE_IDS` arrays.

**Keep `GENRE_OPTIONS`/`GENRE_IDS` in sync with the `"类型"` filter in `res/filters.json`** — they duplicate the same data and are indexed positionally.

### Chapter list (json/chapter_list/mod.rs)
Paginated **ascending**, then the whole list is reversed at the end so chapters appear newest-first. `extract_chapter_number` parses "第N话/章/回/卷/册" (or a leading bare number) from the title into `chapter_number`. Titles ending in `卷` are treated as volumes (scanlator `单行本`) instead of chapters.

### Page images + scan decryption (json/page_list.rs)
The reading request requires anti-bot/browser-like headers: an `X-Requested-Id` timestamp, `Accept: application/json`, and a crafted `_ga_HVJMXGJXFJ` cookie whose value embeds `generate_ga_timestamp()` (a timestamp with a checksum suffix derived from a lookup `TABLE` keyed on the last 3 digits). The response's `scans` field is either a JSON array or an encrypted string; when `isEncode` is true it's decrypted via:
1. SHA-256 key derivation (inputs: `SECRET` + first 8 bytes + `DOMAIN`) to compute offsets that locate hex-encoded key, nonce, and base64 ciphertext within the blob
2. Custom SHA-256-based keystream XORed in 32-byte blocks (`key‖nonce‖block_idx` hashed per block)
3. `miniz_oxide` zlib decompression; plaintext must start with `SC01`

If scan decoding breaks, check `SECRET` (currently a placeholder `DEV_SCAN_SECRET_2026_change_me`) and `DOMAIN` (`hipmh.com`) against the live site. When building `Page`s, entries with `n != 0` (images from the next chapter) are skipped and the `?q=` query param is stripped from image URLs.

### HTML parsing (html/mod.rs)
Manga detail uses `.mg-cover>mip-img` (cover), `h2.mg-title` (title), `.mg-sub-title>a` (authors), `#showmore` (description), `.mg-cate>a` (tags). Ranking pages use `.manga-rank` items with `.manga-rank-cover>a` (href → id), `.manga-rank-cover>a>mip-img` (cover), `.manga-title`. `Viewer::Webtoon` is set for all manga.

### Deep Link Handling
- `/works/{base64_id}--{slug}` — base64-decodes the ID (decodes to `m:{numeric_id}`), used as the manga key
- `/manga/{id}` and `/manga/{id}/{chapter_id}` — direct internal links

## Deploy

Built `package.aix` is placed at `public/sources/zh.hipmh-v1.aix` alongside `public/index.json`/`public/index.min.json` (the Aidoku source list). The `gh-pages` branch serves `index.min.json` + `sources/*.aix` + `icons/*.png` and is generated from the `master` `public/` directory. Repo: `doyayaa/aidoku-source-zh-hipmh`.

**The manifest MUST be the new `{"name": ..., "sources": [...]}` format** (fields `iconURL`/`downloadURL`/`languages`/`contentRating`/`baseURL`/`minAppVersion`). The old flat-array format (`file`/`icon`/`lang`/`nsfw`) causes Aidoku to label the whole source list "旧版图源" (legacy). Keep `public/` and the `gh-pages` branch in sync, including the icon referenced by `iconURL`.

## Key Dependencies

- `aidoku` (git: Aidoku/aidoku-rs) — core framework with `helpers` + `json` features
- `serde`/`serde_json` — JSON parsing
- `base64` — scan and ID encoding
- `sha2` — scan decryption
- `miniz_oxide` — decompress encrypted scans
- `regex` — chapter number extraction from titles
- `chrono` — date handling (if needed)

## Differences from zh.happymh

- `BASE_URL` changed to hipmh.com
- `DOMAIN` in scan decryption changed to `"hipmh.com"`
- Deep link handler added support for `/works/` URL pattern (hipmh's Nuxt frontend format)
- Search referer uses `/search` instead of `/sssearch`
