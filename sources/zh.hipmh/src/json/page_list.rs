use aidoku::{
	Page, Result,
	alloc::{String, Vec, string::ToString as _},
	error,
	imports::net::Request,
	prelude::*,
};
use base64::{Engine as _, engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD}};

const CHAPTER_API: &str = "https://hipapi1.s3file.top/v2/chapter";
const READER_ORIGIN: &str = "https://reader.hipmh.top";
const IMAGE_BASE: &str = "https://hip-tx-";

// Scrambled url-safe base64 alphabet used by the reader (FROM -> TO substitution).
const FROM: &[u8] = b"_-9876543210abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ";
const TO: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";

const TRANSLATE: [u8; 256] = {
	let mut t = [0u8; 256];
	let mut i = 0usize;
	while i < 256 {
		t[i] = i as u8;
		i += 1;
	}
	let mut j = 0usize;
	while j < FROM.len() {
		t[FROM[j] as usize] = TO[j];
		j += 1;
	}
	t
};

pub struct PageList;

impl PageList {
	/// Fetch a chapter's page images from `https://hipapi1.s3file.top/v2/chapter`.
	///
	/// `chapter_key` is the chapter `hid` from `/v1/manga/chapters`, e.g.
	/// `bToyMzQ3NS1jOjc4MzI5-MjM0NzU6ODI1LjAw` (base64 of `m:{mid}-c:{cid}` `-`
	/// base64 of `{mid}:{num}.00`). The API wants an `api_hid` built from the
	/// chapter id alone: base64(`c:{cid}`) padding-stripped `-` seg2.
	pub fn get_pages(_manga_key: String, chapter_key: String) -> Result<Vec<Page>> {
		let api_hid = derive_api_hid(&chapter_key)?;
		let json: serde_json::Value = Request::get(format!("{}?hid={}", CHAPTER_API, api_hid))?
			.header("Origin", READER_ORIGIN)
			.header("Referer", READER_ORIGIN)
			.send()?
			.get_json()?;
		let data = json
			.get("data")
			.ok_or_else(|| error!("Expected data object"))?;
		let images = data
			.get("images")
			.and_then(|v| v.as_str())
			.ok_or_else(|| error!("Expected images string"))?;
		let line = data.get("line").and_then(|v| v.as_i64()).unwrap_or(1);

		let paths = decode_images(images)?;
		let base = format!("{}{}.s3imgs.top", IMAGE_BASE, line);
		let mut pages = Vec::with_capacity(paths.len());
		for path in paths {
			pages.push(Page {
				content: aidoku::PageContent::url(format!("{}{}", base, path)),
				..Default::default()
			});
		}
		Ok(pages)
	}
}

/// Derive the `api_hid` the reading API expects from a chapter `hid`:
/// `base64("c:{cid}")` (padding stripped) + `-` + the second segment.
fn derive_api_hid(chapter_key: &str) -> Result<String> {
	let (seg1, seg2) = chapter_key
		.rsplit_once('-')
		.ok_or_else(|| error!("Invalid chapter key"))?;
	let decoded = decode_b64(seg1)?;
	let decoded = String::from_utf8(decoded).map_err(|_| error!("Invalid hid segment"))?;
	// decoded looks like "m:{mid}-c:{cid}"; we only need the chapter id.
	let cid = decoded
		.rsplit_once("-c:")
		.map(|(_, cid)| cid)
		.ok_or_else(|| error!("Unexpected hid segment format"))?;
	let cid_enc = STANDARD.encode(format!("c:{}", cid));
	Ok(format!("{}-{}", cid_enc.trim_end_matches('='), seg2))
}

/// Decode the encrypted `data.images` payload into the list of relative image paths.
///
/// Pure character transform (no crypto), verified byte-for-byte against the
/// site's obfuscated `chapter-decoder.js`:
/// 1. Strip `qM9` prefix / `Z7` suffix
/// 2. Layout `A + "Vx" + B + "pL0" + K`, recombine as `K + A + B`
/// 3. Split into 7-char chunks, reverse every odd chunk
/// 4. Substitute chars FROM -> TO (scrambled to standard url-safe base64 alphabet)
/// 5. Url-safe base64 (no padding) -> UTF-8 JSON array of relative paths
fn decode_images(encrypted: &str) -> Result<Vec<String>> {
	let input = encrypted.as_bytes();
	if input.len() < 8 || !input.starts_with(b"qM9") || !input.ends_with(b"Z7") {
		bail!("Invalid images payload");
	}
	let inner = &input[3..input.len() - 2];
	let total = inner.len() - 5;
	let k_len = total / 3;
	let a_len = (total - k_len) / 2;
	let b_len = total - k_len - a_len;

	// Sanity-check the "Vx" / "pL0" separators before slicing.
	if inner[a_len] != b'V'
		|| inner[a_len + 1] != b'x'
		|| inner[a_len + 2 + b_len] != b'p'
		|| inner[a_len + 2 + b_len + 1] != b'L'
		|| inner[a_len + 2 + b_len + 2] != b'0'
	{
		bail!("Unexpected images layout");
	}
	let seg_a = &inner[..a_len];
	let seg_b = &inner[a_len + 2..a_len + 2 + b_len];
	let seg_k = &inner[a_len + 2 + b_len + 3..];

	let mut combined = Vec::with_capacity(inner.len());
	combined.extend_from_slice(seg_k);
	combined.extend_from_slice(seg_a);
	combined.extend_from_slice(seg_b);

	let mut substituted = Vec::with_capacity(combined.len());
	for (idx, chunk) in combined.chunks(7).enumerate() {
		if idx % 2 == 1 {
			for &b in chunk.iter().rev() {
				substituted.push(TRANSLATE[b as usize]);
			}
		} else {
			for &b in chunk {
				substituted.push(TRANSLATE[b as usize]);
			}
		}
	}

	let decoded = URL_SAFE_NO_PAD
		.decode(&substituted)
		.map_err(|_| error!("Failed to decode images base64"))?;
	let json_str = String::from_utf8(decoded).map_err(|_| error!("Invalid images JSON"))?;
	let parsed: serde_json::Value =
		serde_json::from_str(&json_str).map_err(|_| error!("Failed to parse images JSON"))?;
	let arr = parsed
		.as_array()
		.ok_or_else(|| error!("Expected images array"))?;

	let mut paths = Vec::with_capacity(arr.len());
	for item in arr {
		if let Some(path) = item.as_str() {
			paths.push(path.to_string());
		}
	}
	Ok(paths)
}

/// Decode base64 that may have its padding stripped (the site strips `=`).
fn decode_b64(input: &str) -> Result<Vec<u8>> {
	let mut padded = input.to_string();
	let rem = padded.len() % 4;
	if rem != 0 {
		for _ in 0..(4 - rem) {
			padded.push('=');
		}
	}
	STANDARD
		.decode(padded.as_bytes())
		.map_err(|_| error!("Invalid base64"))
}
