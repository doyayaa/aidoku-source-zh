#![no_std]

mod html;
mod json;

use aidoku::{
	Chapter, DeepLinkHandler, DeepLinkResult, ImageRequestProvider, Listing, ListingProvider,
	Manga, MangaPageResult, Page, Result, Source,
	alloc::{String, Vec, string::ToString as _},
	helpers::uri::encode_uri,
	imports::net::Request,
	prelude::*,
};
use html::MangaPage as _;

pub const BASE_URL: &str = "https://m.hipmh.com";
const API_URL: &str = "https://hipapi1.s3file.top/v1/mangas";

struct Hipmh;

impl Source for Hipmh {
	fn new() -> Self {
		Self
	}

	fn get_search_manga_list(
		&self,
		query: Option<String>,
		page: i32,
		filters: Vec<aidoku::FilterValue>,
	) -> Result<MangaPageResult> {
		// A text filter is treated as a search query.
		let text = filters.iter().find_map(|f| match f {
			aidoku::FilterValue::Text { value, .. } if !value.is_empty() => Some(value.clone()),
			_ => None,
		});
		let query = text.or(query).filter(|q| !q.trim().is_empty());

		let url = match query {
			Some(q) => format!(
				"https://hipapi1.s3file.top/v1/search?q={}&page={}&page_size=20",
				encode_uri(&q),
				page
			),
			None => build_mangas_filter_url(page, &filters),
		};

		let json: serde_json::Value = Request::get(url)?
			.header("Origin", BASE_URL)
			.header("Referer", BASE_URL)
			.send()?
			.get_json()?;

		parse_manga_list(json, page)
	}

	fn get_manga_update(
		&self,
		mut manga: Manga,
		needs_details: bool,
		needs_chapters: bool,
	) -> Result<Manga> {
		if needs_details {
			let works_url = format!("{}/works/{}", BASE_URL, key_to_works_id(&manga.key));
			let doc = Request::get(works_url.clone())?
				.header("Origin", BASE_URL)
				.html()?;
			doc.update_details(&mut manga)?;
			manga.url = Some(works_url);
		}

		if needs_chapters {
			manga.chapters = Some(json::chapter_list::ChapterList::get_chapters(&manga.key)?);
		}

		Ok(manga)
	}

	fn get_page_list(&self, manga: Manga, chapter: Chapter) -> Result<Vec<Page>> {
		json::page_list::PageList::get_pages(manga.key, chapter.key)
	}
}

impl ImageRequestProvider for Hipmh {
	fn get_image_request(
		&self,
		url: String,
		_context: Option<aidoku::PageContext>,
	) -> Result<Request> {
		Ok(Request::get(url)?.header("Referer", BASE_URL))
	}
}

impl DeepLinkHandler for Hipmh {
	fn handle_deep_link(&self, url: String) -> Result<Option<DeepLinkResult>> {
		// Handle URLs like:
		// https://m.hipmh.com/works/bToxNTAzMQ--xia-ri-chong-sheng
		// https://m.hipmh.com/manga/{id}
		let url = url.trim_start_matches(BASE_URL);
		let mut splits = url.split('/').skip(1);

		match splits.next() {
			Some("works") => {
				// Extract manga_code from /works/{base64_encode}--{slug}
				// The base64 part encodes "m:{numeric_id}"
				if let Some(work_id) = splits.next() {
					// hipmh uses base64 encoding for IDs in the URL
					// We extract the raw part and pass it as the manga key
					let manga_code = work_id.split("--").next().unwrap_or(work_id);
					return Ok(Some(DeepLinkResult::Manga {
						key: decode_work_id(manga_code),
					}));
				}
				Ok(None)
			}
			Some("manga") => match (splits.next(), splits.next()) {
				(Some(manga_id), None) => Ok(Some(DeepLinkResult::Manga {
					key: manga_id.into(),
				})),
				(Some(manga_id), Some(chapter_id)) => Ok(Some(DeepLinkResult::Chapter {
					manga_key: manga_id.into(),
					key: chapter_id.into(),
				})),
				_ => Ok(None),
			},
			_ => Ok(None),
		}
	}
}

impl ListingProvider for Hipmh {
	fn get_manga_list(&self, listing: Listing, page: i32) -> Result<MangaPageResult> {
		let url = match listing.id.as_str() {
			"popularity" => format!("{}?sort=popular&page={}&per_page=18", API_URL, page),
			"weekly" => format!("{}?sort=weekly&page={}&per_page=18", API_URL, page),
			"newReleases" => format!("{}?sort=latest&page={}&per_page=18", API_URL, page),
			"completed" => format!(
				"{}?status=completed&sort=popular&page={}&per_page=18",
				API_URL, page
			),
			"ongoing" => format!(
				"{}?status=ongoing&sort=popular&page={}&per_page=18",
				API_URL, page
			),
			_ => bail!("Invalid listing"),
		};

		let json: serde_json::Value = Request::get(url)?
			.header("Origin", BASE_URL)
			.header("Referer", BASE_URL)
			.send()?
			.get_json()?;

		parse_manga_list(json, page)
	}
}

/// Decode hipmh's base64 work ID format (e.g., "bToxNTAzMQ" -> "m:15031")
/// The base64 decodes to "m:{numeric_id}"
fn decode_work_id(encoded: &str) -> String {
	use base64::{Engine as _, engine::general_purpose::STANDARD};
	match STANDARD.decode(encoded) {
		Ok(bytes) => String::from_utf8(bytes).unwrap_or_else(|_| encoded.into()),
		Err(_) => encoded.into(),
	}
}

/// Convert a hipmh `/works/` slug (e.g. "bToyMzQ3NQ-yi-ren-zhi-xia-tencent-531490-17793")
/// to a manga key by base64-decoding the leading ID segment (decodes to "m:{numeric_id}").
/// Falls back to the raw first segment if decoding fails.
fn works_slug_to_key(slug: &str) -> String {
	let encoded = slug.split('-').next().unwrap_or(slug);
	decode_work_id(encoded)
}

/// Convert a manga key (e.g. "m:23475") back to the leading `/works/` ID segment
/// used in detail-page URLs. The site's canonical form strips base64 padding:
/// `m:23475` -> `bToyMzQ3NQ`, NOT `bToyMzQ3NQ==` (a padded path 404s on the
/// Nuxt router). `STANDARD.encode` emits the padding, so trim it here.
fn key_to_works_id(key: &str) -> String {
	use base64::{Engine as _, engine::general_purpose::STANDARD};
	STANDARD.encode(key).trim_end_matches('=').to_string()
}

/// Parse a `/v1/mangas` or `/v1/search` response into a `MangaPageResult`.
/// Handles both `data.items` (listings) and `data.data` (search) item arrays,
/// and both `mid` (listings) and `id` (search) item key fields.
fn parse_manga_list(json: serde_json::Value, page: i32) -> Result<MangaPageResult> {
	let data = json
		.get("data")
		.ok_or_else(|| error!("Expected data object"))?;
	let items = data
		.get("items")
		.or_else(|| data.get("data"))
		.and_then(|v| v.as_array())
		.ok_or_else(|| error!("Expected items array"))?;
	let total_pages = data.get("total_pages").and_then(|v| v.as_i64()).unwrap_or(1);

	let mut mangas: Vec<Manga> = Vec::new();
	for item in items {
		let item = match item.as_object() {
			Some(item) => item,
			None => continue,
		};
		let mid = item
			.get("mid")
			.or_else(|| item.get("id"))
			.and_then(|v| v.as_str())
			.unwrap_or_default();
		let title = item
			.get("title")
			.and_then(|v| v.as_str())
			.unwrap_or_default()
			.to_string();
		let cover = item
			.get("vertical_image_url")
			.or_else(|| item.get("cover_image_url"))
			.and_then(|v| v.as_str())
			.map(|u| {
				if u.starts_with("http") {
					u.to_string()
				} else {
					format!("https://cover.s3imgs.top{}", u)
				}
			})
			.unwrap_or_default();
		mangas.push(Manga {
			key: works_slug_to_key(mid),
			cover: Some(cover),
			title,
			..Default::default()
		});
	}

	Ok(MangaPageResult {
		entries: mangas,
		has_next_page: page < total_pages as i32,
	})
}

/// Build a `/v1/mangas` browse URL from the Chinese filter selects.
/// Only the 状态 (status) filter maps directly (`status=ongoing|completed`);
/// the 类型/地区/genre filters need numeric IDs that aren't exposed by the API,
/// so they're skipped to avoid 400 errors.
fn build_mangas_filter_url(page: i32, filters: &[aidoku::FilterValue]) -> String {
	let mut url = format!("{}?page={}&per_page=18", API_URL, page);
	for filter in filters {
		let aidoku::FilterValue::Select { id, value } = filter else {
			continue;
		};
		if id.as_str() == "状态" {
			let status = match value.as_str() {
				"0" => "ongoing",
				"1" => "completed",
				_ => continue,
			};
			url.push_str(&format!("&status={}", status));
		}
	}
	url
}

register_source!(
	Hipmh,
	ListingProvider,
	ImageRequestProvider,
	DeepLinkHandler
);