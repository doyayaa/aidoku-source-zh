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
			let manga_page = Url::manga(manga.key.clone()).request()?.html()?;
			manga_page.update_details(&mut manga)?;
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
			"day" => format!("{}/rank/day?page={}", BASE_URL, page),
			"week" => format!("{}/rank/week?page={}", BASE_URL, page),
			"month" => format!("{}/rank/month?page={}", BASE_URL, page),
			"dayBookcasesOne" => format!("{}/rank/dayBookcasesOne?page={}", BASE_URL, page),
			"weekBookcasesOne" => format!("{}/rank/weekBookcasesOne?page={}", BASE_URL, page),
			"monthBookcasesOne" => format!("{}/rank/monthBookcasesOne?page={}", BASE_URL, page),
			"voteNumMonthRank" => format!("{}/rank/voteNumMonthRank?page={}", BASE_URL, page),
			"voteRank" => format!("{}/rank/voteRank?page={}", BASE_URL, page),
			"latest" => return self.get_search_manga_list(None, page, Vec::new()),
			_ => bail!("Invalid listing"),
		};

		let html = Request::get(url)?.header("Origin", BASE_URL).html()?;

		html.manga_page_result()
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

register_source!(
	Hipmh,
	ListingProvider,
	ImageRequestProvider,
	DeepLinkHandler
);