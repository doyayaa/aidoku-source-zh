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
	pub fn get_chapters(manga_key: &str) -> Result<Vec<Chapter>> {
		let mid = manga_key.strip_prefix("m:").unwrap_or(manga_key);
		let mut all_chapters: Vec<Chapter> = Vec::new();
		let mut page = 1;

		loop {
			let url = format!(
				"https://hipapi1.s3file.top/v1/manga/chapters?mid={}&page={}&per_page=50&order=desc",
				mid, page
			);
			let json: serde_json::Value = Request::get(url)?
				.header("Origin", BASE_URL)
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

			if page >= total_pages as i32 {
				break;
			}
			page += 1;
		}

		Ok(all_chapters)
	}
}
