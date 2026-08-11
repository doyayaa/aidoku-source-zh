# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Aidoku manga source extension for **嬉皮漫画** (https://m.hipmh.com). Based on the `zh.happymh` source from [Aidoku-Community/sources](https://github.com/Aidoku-Community/sources), adapted for the hipmh.com domain.

## Build

Requires **Rust nightly** (edition 2024) with `wasm32-unknown-unknown` target.

```bash
# Install wasm target
rustup target add wasm32-unknown-unknown

# Build (bash)
./build.sh

# Build (PowerShell)
./build.ps1
```

Output: `package.aix` — a zip file containing `main.wasm` and `res/*` files. This is loaded by the Aidoku app.

## Architecture

The source implements the Aidoku trait-based API (newer API from Aidoku 0.7+, NOT the older `#[get_manga_list]` attribute-based API used in Skittyblock/aidoku-community-sources).

### Module Structure

```
src/
├── lib.rs              # Entry point: Source, ImageRequestProvider, DeepLinkHandler, ListingProvider
├── net/mod.rs          # URL construction, filter parsing, API request building
├── html/mod.rs         # HTML scraper: manga detail parsing, ranking page parsing
└── json/
    ├── mod.rs          # Module declarations
    ├── chapter_list/mod.rs  # Chapter list API (paginated, asc then reversed)
    └── page_list.rs    # Page/image list with encrypted scan decryption
```

### API Endpoints

| Purpose | Method | Path | Notes |
|---------|--------|------|-------|
| Browse/Filter | GET | `/apis/c/index?&order=last_date&genre={g}&area={a}&audience={au}&series_status={s}&pn={p}` | Returns JSON |
| Search | POST | `/v2.0/apis/manga/ssearch` | Body: `searchkey={q}&v=v2.13&page={p}` |
| Manga Detail | GET | `/manga/{id}` | Returns HTML |
| Chapter List | GET | `/v2.0/apis/manga/chapterByPage?code={id}&page={p}&lang=cn&order=asc` | Paginated, `isEnd` field signals end |
| Page Images | GET | `/v2.0/apis/manga/reading?code={mid}&cid={chid}&v=v4.300101&_t={ts}` | Encrypted scans |
| Rankings | GET | `/rank/{type}?page={p}` | Returns HTML, parsed by html module |

### Scan Decryption (page_list.rs)

Chapter images may be encrypted. Decryption uses:
1. SHA-256 for key derivation (domain `hipmh.com` is part of the key material)
2. Custom stream cipher (SHA-256-based keystream XORed in 32-byte blocks)
3. zlib decompression (`miniz_oxide`) after decryption

If the scan decryption breaks, the `SECRET` constant and `DOMAIN` should be checked against the current site.

### Deep Link Handling

Handles two URL patterns:
- `/works/{base64_id}--{slug}` — hipmh Nuxt frontend links (decodes base64 ID)
- `/manga/{id}` — internal manga page links

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