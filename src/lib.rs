#![no_std]

mod html;
mod json;
mod net;

use aidoku::{
	Chapter, DeepLinkHandler, DeepLinkResult, ImageRequestProvider, Listing, ListingProvider,
	Manga, MangaPageResult, Page, Result, Source,
	alloc::{String, Vec, string::ToString as _},
	imports::net::Request,
	prelude::*,
};
use html::MangaPage as _;
use net::Url;

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
		let url = Url::from_query_or_filters(query.as_deref(), page, &filters)?;
		let json: serde_json::Value = url.request()?.send()?.get_json()?;

		enum ArraySource<'a> {
			Borrowed(&'a [serde_json::Value]),
			Owned(Vec<serde_json::Value>),
		}

		let mut list_vec: Option<ArraySource> = None;

		fn try_extract<'a>(v: &'a serde_json::Value) -> Option<ArraySource<'a>> {
			if let Some(arr) = v.as_array() {
				return Some(ArraySource::Borrowed(arr));
			}
			if let Some(s) = v.as_str()
				&& let Ok(parsed) = serde_json::from_str::<serde_json::Value>(s)
				&& let Some(arr) = parsed.as_array()
			{
				return Some(ArraySource::Owned(arr.clone()));
			}
			None
		}

		// 1) data.items
		if let Some(data_obj) = json.get("data") {
			if let Some(items) = data_obj.get("items") {
				list_vec = try_extract(items);
			} else if let Some(arr) = data_obj.as_array() {
				list_vec = Some(ArraySource::Borrowed(arr));
			}
		}

		// 2) top-level items
		if list_vec.is_none()
			&& let Some(items) = json.get("items")
		{
			list_vec = try_extract(items);
		}

		// 3) payload.items
		if list_vec.is_none()
			&& let Some(payload) = json.get("payload")
			&& let Some(items) = payload.get("items")
		{
			list_vec = try_extract(items);
		}

		let list = match list_vec {
			Some(ArraySource::Borrowed(v)) => v,
			Some(ArraySource::Owned(ref v)) => v,
			None => bail!("Expected items array in search response"),
		};

		let mut mangas: Vec<Manga> = Vec::new();

		for item in list {
			let item = match item.as_object() {
				Some(item) => item,
				None => continue,
			};
			let id = item
				.get("manga_code")
				.and_then(|v| v.as_str())
				.unwrap_or_default()
				.to_string();
			let cover = item
				.get("cover")
				.and_then(|v| v.as_str())
				.unwrap_or_default()
				.to_string();
			let title = item
				.get("name")
				.and_then(|v| v.as_str())
				.unwrap_or_default()
				.to_string();
			mangas.push(Manga {
				key: id,
				cover: Some(cover),
				title,
				..Default::default()
			});
		}

		Ok(MangaPageResult {
			entries: mangas.clone(),
			has_next_page: !mangas.is_empty(),
		})
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

		let data = json
			.get("data")
			.ok_or_else(|| error!("Expected data object"))?;
		let items = data
			.get("items")
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
/// used in detail-page URLs (base64-encodes to "bToyMzQ3NQ").
fn key_to_works_id(key: &str) -> String {
	use base64::{Engine as _, engine::general_purpose::STANDARD};
	STANDARD.encode(key)
}

register_source!(
	Hipmh,
	ListingProvider,
	ImageRequestProvider,
	DeepLinkHandler
);