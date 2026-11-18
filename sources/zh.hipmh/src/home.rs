use crate::{BASE_URL, Hipmh, listing_url, parse_manga_list, settings};
use aidoku::{
	Home, HomeComponent, HomeComponentValue, HomeLayout, HomePartialResult, Listing, ListingKind,
	MangaPageResult, Result,
	alloc::{Vec, vec},
	imports::{net::Request, std::send_partial_result},
};

fn send_component(component: HomeComponent) {
	send_partial_result(&HomePartialResult::Component(component));
}

fn listing_kind() -> ListingKind {
	if settings::get_list_viewer() {
		ListingKind::List
	} else {
		ListingKind::Default
	}
}

impl Home for Hipmh {
	/// PicACG-style home: banner (人气榜) + ranked list (本周热门) + scrollers, streamed progressively.
	fn get_home(&self) -> Result<HomeLayout> {
		send_partial_result(&HomePartialResult::Layout(HomeLayout {
			components: vec![
				HomeComponent {
					title: Some("人气榜".into()),
					subtitle: None,
					value: HomeComponentValue::empty_big_scroller(),
				},
				HomeComponent {
					title: Some("本周热门".into()),
					subtitle: None,
					value: HomeComponentValue::empty_manga_list(),
				},
				HomeComponent {
					title: Some("最新上架".into()),
					subtitle: None,
					value: HomeComponentValue::empty_scroller(),
				},
				HomeComponent {
					title: Some("完结".into()),
					subtitle: None,
					value: HomeComponentValue::empty_scroller(),
				},
				HomeComponent {
					title: Some("连载".into()),
					subtitle: None,
					value: HomeComponentValue::empty_scroller(),
				},
			],
		}));

		// Fetch all five listings concurrently; results come back in request order.
		let ids = ["popularity", "weekly", "newReleases", "completed", "ongoing"];
		let mut requests = Vec::with_capacity(ids.len());
		for id in ids {
			requests.push(
				Request::get(listing_url(id, 1)?)?
					.header("Origin", BASE_URL)
					.header("Referer", BASE_URL),
			);
		}
		let parsed: Vec<Result<MangaPageResult>> = Request::send_all(requests)
			.into_iter()
			.map(|resp| -> Result<MangaPageResult> {
				let json: serde_json::Value = resp?.get_json()?;
				parse_manga_list(json, 1)
			})
			.collect();

		let kind = listing_kind();

		if let Some(Ok(r)) = parsed.first()
			&& !r.entries.is_empty()
		{
			send_component(HomeComponent {
				title: Some("人气榜".into()),
				subtitle: None,
				value: HomeComponentValue::BigScroller {
					entries: r.entries.clone(),
					auto_scroll_interval: Some(8.0),
				},
			});
		}

		if let Some(Ok(r)) = parsed.get(1)
			&& !r.entries.is_empty()
		{
			send_component(HomeComponent {
				title: Some("本周热门".into()),
				subtitle: None,
				value: HomeComponentValue::MangaList {
					ranking: true,
					page_size: Some(3),
					entries: r.entries.clone().into_iter().map(|m| m.into()).collect(),
					listing: Some(Listing {
						id: "weekly".into(),
						name: "本周热门".into(),
						kind,
					}),
				},
			});
		}

		if let Some(Ok(r)) = parsed.get(2)
			&& !r.entries.is_empty()
		{
			send_component(HomeComponent {
				title: Some("最新上架".into()),
				subtitle: None,
				value: HomeComponentValue::Scroller {
					entries: r.entries.clone().into_iter().map(|m| m.into()).collect(),
					listing: Some(Listing {
						id: "newReleases".into(),
						name: "最新上架".into(),
						kind,
					}),
				},
			});
		}

		if let Some(Ok(r)) = parsed.get(3)
			&& !r.entries.is_empty()
		{
			send_component(HomeComponent {
				title: Some("完结".into()),
				subtitle: None,
				value: HomeComponentValue::Scroller {
					entries: r.entries.clone().into_iter().map(|m| m.into()).collect(),
					listing: Some(Listing {
						id: "completed".into(),
						name: "完结".into(),
						kind,
					}),
				},
			});
		}

		if let Some(Ok(r)) = parsed.get(4)
			&& !r.entries.is_empty()
		{
			send_component(HomeComponent {
				title: Some("连载".into()),
				subtitle: None,
				value: HomeComponentValue::Scroller {
					entries: r.entries.clone().into_iter().map(|m| m.into()).collect(),
					listing: Some(Listing {
						id: "ongoing".into(),
						name: "连载".into(),
						kind,
					}),
				},
			});
		}

		Ok(HomeLayout::default())
	}
}