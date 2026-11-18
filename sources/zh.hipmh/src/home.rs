use crate::{BASE_URL, Hipmh, settings, works_slug_to_key};
use aidoku::{
	Home, HomeComponent, HomeComponentValue, HomeLayout, Link, LinkValue, Listing, ListingKind,
	Manga, Result,
	alloc::{String, Vec},
	imports::net::Request,
};

// The home page is Astro SSR; a mobile UA gets the full server-rendered layout.
const MOBILE_UA: &str = "Mozilla/5.0 (Linux; Android 13; Pixel 7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/125.0.0.0 Mobile Safari/537.36";

fn listing_kind() -> ListingKind {
	if settings::get_list_viewer() {
		ListingKind::List
	} else {
		ListingKind::Default
	}
}

fn works_key(href: &str) -> Option<String> {
	Some(works_slug_to_key(href.strip_prefix("/works/")?))
}

impl Home for Hipmh {
	/// Mirrors the site's recommendation page (PicACG/copymanga pattern): scrape the
	/// SSR home HTML into a top banner ImageScroller plus the manga sections.
	fn get_home(&self) -> Result<HomeLayout> {
		let doc = Request::get(BASE_URL)?
			.header("User-Agent", MOBILE_UA)
			.header("Origin", BASE_URL)
			.header("Referer", BASE_URL)
			.html()?;
		let mut components: Vec<HomeComponent> = Vec::new();
		let kind = listing_kind();

		// Top sliding banner -> ImageScroller (skip the duplicated clone slides).
		let banners: Vec<Link> = doc
			.select(".banner-slide")
			.map(|slides| {
				slides
					.filter_map(|slide| {
						if slide.has_class("banner-slide-clone") {
							return None;
						}
						let href = slide.select_first("a.banner-link")?.attr("href")?;
						let key = works_key(&href)?;
						let image_url = slide
							.select_first("img.banner-image")
							.and_then(|i| i.attr("src"))?;
						let title = slide
							.select_first("h3.banner-title")
							.and_then(|t| t.text())
							.unwrap_or_default();
						let subtitle = slide
							.select_first("p.banner-sub-title")
							.and_then(|t| t.text());
						Some(Link {
							title: title.clone(),
							subtitle,
							image_url: Some(image_url),
							value: Some(LinkValue::Manga(Manga {
								key,
								title,
								..Default::default()
							})),
						})
					})
					.collect()
			})
			.unwrap_or_default();
		if !banners.is_empty() {
			components.push(HomeComponent {
				title: None,
				subtitle: None,
				value: HomeComponentValue::ImageScroller {
					links: banners,
					auto_scroll_interval: Some(5.0),
					width: Some(300),
					height: Some(150),
				},
			});
		}

		// 近期更新: cards carry the title in aria-label.
		if let Some(section) = doc.select_first(".recent-updates-section") {
			let entries: Vec<Link> = section
				.select("a[href^='/works/']")
				.map(|cards| {
					cards
						.filter_map(|a| {
							let key = works_key(&a.attr("href")?)?;
							let title = a.attr("aria-label").unwrap_or_default();
							let cover = a.select_first("img").and_then(|i| i.attr("src"));
							Some(
								Manga {
									key,
									title,
									cover,
									..Default::default()
								}
								.into(),
							)
						})
						.collect()
				})
				.unwrap_or_default();
			if !entries.is_empty() {
				components.push(HomeComponent {
					title: Some("近期更新".into()),
					subtitle: None,
					value: HomeComponentValue::Scroller {
						entries,
						listing: None,
					},
				});
			}
		}

		// Remaining sections in site order; map each heading to its listing.
		if let Some(sections) = doc.select(".manga-section") {
			for section in sections {
				let heading = section
					.select_first("h2")
					.and_then(|h| h.text())
					.unwrap_or_default();
				let listing = match heading.trim() {
					"本週熱門" => Some(("weekly", "本週熱門")),
					"人氣排名" => Some(("popularity", "人氣排名")),
					"高分韓漫" => Some(("highScoreKorean", "高分韓漫")),
					"最新上架" => Some(("newReleases", "最新上架")),
					"完結推薦" => Some(("completed", "完結推薦")),
					_ => None,
				};
				let entries: Vec<Link> = section
					.select("a.manga-card-link")
					.map(|cards| {
						cards
							.filter_map(|a| {
								let key = works_key(&a.attr("href")?)?;
								let title = a
									.select_first("h3.manga-card-title")
									.and_then(|t| t.text())
									.or_else(|| a.attr("aria-label"))
									.unwrap_or_default();
								let cover = a
									.select_first("img.manga-card-image")
									.and_then(|i| i.attr("src"));
								Some(
									Manga {
										key,
										title,
										cover,
										..Default::default()
									}
									.into(),
								)
							})
							.collect()
					})
					.unwrap_or_default();
				if entries.is_empty() {
					continue;
				}
				components.push(HomeComponent {
					title: Some(heading.clone()),
					subtitle: None,
					value: HomeComponentValue::Scroller {
						entries,
						listing: listing.map(|(id, name)| Listing {
							id: id.into(),
							name: name.into(),
							kind,
						}),
					},
				});
			}
		}

		Ok(HomeLayout { components })
	}
}