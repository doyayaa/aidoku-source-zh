use crate::BASE_URL;
use aidoku::{
	Chapter, Result,
	alloc::{Vec, string::ToString as _},
	error,
	imports::net::Request,
	prelude::format,
};

pub struct ChapterList;

impl ChapterList {
	/// Fetch chapters from `https://hipapi1.s3file.top/v1/manga/chapters`.
	/// The manga key is `m:{numeric_id}`; the API wants the numeric id.
	///
	/// The API caps `per_page` at 50, so a long manga spans many pages. Page 1
	/// is fetched first to learn `total_pages`, then the remaining pages are
	/// fetched concurrently via `Request::send_all` (results come back in
	/// request order, so `order=desc` newest-first ordering is preserved).
	pub fn get_chapters(manga_key: &str) -> Result<Vec<Chapter>> {
		let mid = manga_key.strip_prefix("m:").unwrap_or(manga_key);
		let base = format!("https://hipapi1.s3file.top/v1/manga/chapters?mid={}", mid);

		// Page 1 first: it reports total_pages.
		let url1 = format!("{}&page=1&per_page=50&order=desc", base);
		let page1: serde_json::Value = Request::get(url1)?
			.header("Origin", BASE_URL)
			.send()?
			.get_json()?;
		let total_pages = page1
			.get("data")
			.and_then(|d| d.get("total_pages"))
			.and_then(|v| v.as_i64())
			.map(|n| n.max(1))
			.unwrap_or(1) as usize;

		// Fire off the remaining pages concurrently.
		let mut pages: Vec<serde_json::Value> = Vec::with_capacity(total_pages);
		pages.push(page1);
		let mut requests: Vec<Request> = Vec::with_capacity(total_pages.saturating_sub(1));
		for page in 2..=total_pages {
			let url = format!("{}&page={}&per_page=50&order=desc", base, page);
			requests.push(Request::get(url)?.header("Origin", BASE_URL));
		}
		for response in Request::send_all(requests) {
			let json: serde_json::Value = response?.get_json()?;
			pages.push(json);
		}

		let mut all_chapters: Vec<Chapter> = Vec::new();
		for json in pages {
			let data = json
				.get("data")
				.ok_or_else(|| error!("Expected data object"))?;
			let items = data
				.get("items")
				.and_then(|v| v.as_array())
				.ok_or_else(|| error!("Expected items array"))?;
			for item in items {
				let item = match item.as_object() {
					Some(item) => item,
					None => continue,
				};
				let hid = item
					.get("hid")
					.and_then(|v| v.as_str())
					.unwrap_or_default()
					.to_string();
				let title = item
					.get("title")
					.and_then(|v| v.as_str())
					.unwrap_or_default()
					.to_string();
				let chapter_number = item
					.get("chapter_number")
					.and_then(|v| v.as_f64())
					.unwrap_or(0.0);

				all_chapters.push(Chapter {
					key: hid.clone(),
					title: Some(title),
					chapter_number: (chapter_number > 0.0).then_some(chapter_number as f32),
					url: Some(format!("{}/chapter/go?hid={}&m={}", BASE_URL, hid, mid)),
					..Default::default()
				});
			}
		}

		Ok(all_chapters)
	}
}
