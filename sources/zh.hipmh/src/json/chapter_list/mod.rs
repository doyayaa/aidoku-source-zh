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
				let date_uploaded = item
					.get("updated_at")
					.or_else(|| item.get("created_at"))
					.and_then(|v| v.as_str())
					.and_then(parse_iso8601_to_epoch);
				// Each chapter carries its own cover (`cover_image_url`, a
				// relative `/tx/chapter/...` path on the cover CDN). Aidoku
				// renders it as the thumbnail left of the chapter title.
				let thumbnail = item
					.get("cover_image_url")
					.and_then(|v| v.as_str())
					.map(|u| {
						if u.starts_with("http") {
							u.to_string()
						} else {
							format!("https://cover.s3imgs.top{}", u)
						}
					});

				all_chapters.push(Chapter {
					key: hid.clone(),
					title: Some(title),
					chapter_number: (chapter_number > 0.0).then_some(chapter_number as f32),
					date_uploaded,
					thumbnail,
					url: Some(format!("{}/chapter/go?hid={}&m={}", BASE_URL, hid, mid)),
					..Default::default()
				});
			}
		}

		Ok(all_chapters)
	}
}

/// Parse an ISO-8601 timestamp (e.g. "2026-08-07T10:31:40.275232Z") into epoch
/// seconds. The API always emits UTC with a `Z` suffix; the fractional seconds
/// and timezone are ignored (fixed-field slice at indices 0..19). Returns
/// `None` on malformed input.
fn parse_iso8601_to_epoch(s: &str) -> Option<i64> {
	if s.len() < 19 {
		return None;
	}
	let year = s.get(0..4)?.parse::<i64>().ok()?;
	let month = s.get(5..7)?.parse::<u32>().ok()?;
	let day = s.get(8..10)?.parse::<u32>().ok()?;
	let hour = s.get(11..13)?.parse::<i64>().ok()?;
	let minute = s.get(14..16)?.parse::<i64>().ok()?;
	let second = s.get(17..19)?.parse::<i64>().ok()?;

	let days = days_from_civil(year, month, day);
	Some(days * 86_400 + hour * 3_600 + minute * 60 + second)
}

/// Days since the Unix epoch for a proleptic Gregorian civil date
/// (Howard Hinnant's `days_from_civil`).
fn days_from_civil(y: i64, m: u32, d: u32) -> i64 {
	let y = if m <= 2 { y - 1 } else { y };
	let era = if y >= 0 { y } else { y - 399 } / 400;
	let yoe = y - era * 400;
	let mp = (m as i64 + 9) % 12;
	let doy = (153 * mp + 2) / 5 + d as i64 - 1;
	let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
	era * 146_097 + doe - 719_468
}

#[cfg(test)]
mod tests {
	use super::parse_iso8601_to_epoch;
	use aidoku_test::aidoku_test;

	#[aidoku_test]
	fn parses_iso8601_with_fraction_and_z() {
		// 2026-08-07T10:31:40Z in epoch seconds (fraction + Z ignored).
		assert_eq!(
			parse_iso8601_to_epoch("2026-08-07T10:31:40.275232Z"),
			Some(1_786_098_700)
		);
		// Malformed / too-short input.
		assert_eq!(parse_iso8601_to_epoch("2026-08-07"), None);
		assert_eq!(parse_iso8601_to_epoch("not-a-date"), None);
	}
}
