use crate::BASE_URL;
use aidoku::{
	alloc::{string::ToString as _, vec, String, Vec},
	imports::{html::Document, net::Request},
	prelude::*,
	Manga, Result, Viewer,
};

pub trait MangaPage {
	fn update_details(&self, manga: &mut Manga) -> Result<()>;
}

impl MangaPage for Document {
	fn update_details(&self, manga: &mut Manga) -> Result<()> {
		let url = format!("{}/manga/{}", BASE_URL, manga.key);
		let html = Request::get(url.clone())?
			.header("Origin", BASE_URL)
			.html()?;

		manga.cover = html
			.select_first(".mg-cover>mip-img")
			.and_then(|e| e.attr("src"));
		manga.title = html
			.select_first("h2.mg-title")
			.and_then(|e| e.text())
			.unwrap_or_default();
		let author = html
			.select(".mg-sub-title>a")
			.map(|elements| {
				elements
					.filter_map(|a| a.text())
					.collect::<Vec<String>>()
					.join(", ")
			})
			.unwrap_or_default();
		let description = html
			.select_first("#showmore")
			.and_then(|e| e.text())
			.map(|t| t.trim().to_string())
			.unwrap_or_default();
		let categories = html
			.select(".mg-cate>a")
			.map(|elements| elements.filter_map(|a| a.text()).collect::<Vec<String>>())
			.unwrap_or_default();

		manga.authors = Some(vec![author]);
		manga.description = Some(description);
		manga.tags = Some(categories);
		manga.viewer = Viewer::Webtoon;
		manga.url = Some(url);

		Ok(())
	}
}
