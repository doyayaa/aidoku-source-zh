use aidoku::{
	Manga, MangaStatus, Result, Viewer,
	alloc::{String, Vec, string::ToString as _, vec},
	imports::html::Document,
};

pub trait MangaPage {
	fn update_details(&self, manga: &mut Manga) -> Result<()>;
}

impl MangaPage for Document {
	/// Parse a hipmh `/works/{base64_id}` detail page.
	/// The container div carries `data-manga-id`, `data-manga-title`, `data-cover-url`;
	/// description sits in `#d-info-content p`; authors/tags are links; status is a link
	/// to `/ongoing` (連載中) or `/completed` (完結).
	fn update_details(&self, manga: &mut Manga) -> Result<()> {
		manga.title = self
			.select_first("[data-manga-title]")
			.and_then(|e| e.attr("data-manga-title"))
			.unwrap_or_else(|| manga.title.clone());
		manga.cover = self
			.select_first("[data-cover-url]")
			.and_then(|e| e.attr("data-cover-url"));
		let author = self
			.select("a[href^='/author/']")
			.map(|elements| {
				elements
					.filter_map(|a| a.text())
					.collect::<Vec<String>>()
					.join(", ")
			})
			.unwrap_or_default();
		let description = self
			.select_first("#d-info-content p")
			.and_then(|e| e.text())
			.map(|t| t.trim().to_string())
			.unwrap_or_default();
		let categories = self
			.select("a[href^='/genre/']")
			.map(|elements| elements.filter_map(|a| a.text()).collect::<Vec<String>>())
			.unwrap_or_default();

		manga.authors = Some(vec![author]);
		manga.description = Some(description);
		manga.tags = Some(categories);
		manga.status = if self.select_first("a[href='/completed']").is_some() {
			MangaStatus::Completed
		} else if self.select_first("a[href='/ongoing']").is_some() {
			MangaStatus::Ongoing
		} else {
			MangaStatus::Unknown
		};
		manga.viewer = Viewer::Webtoon;

		Ok(())
	}
}
