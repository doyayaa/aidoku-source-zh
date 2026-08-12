use crate::BASE_URL;
use aidoku::{
	FilterValue, Result,
	alloc::{String, string::ToString as _},
	helpers::uri::encode_uri,
	imports::net::Request,
	prelude::*,
};
use core::fmt::{Display, Formatter, Result as FmtResult};

/// 作品分类 (sortid) value -> the site's named category slug. The numeric
/// sortid only works for some categories (科幻未来 sortid=8 is flaky, 奇异幻想
/// sortid=9 404s), so the 作品分类 filter uses the site's canonical named
/// category URLs `/filter/{slug}/{page}.html` instead.
const CATEGORY_SLUGS: [&str; 13] = [
	"", // 0 全部 (no category)
	"FantasyAdventure", // 1 奇幻冒险
	"Action", // 2 战斗热血
	"SuspenseHorror", // 3 悬疑惊悚
	"SchoolLife", // 4 校园青春
	"Romance", // 5 爱情浪漫
	"Workplace", // 6 职场都市
	"Historical", // 7 历史文化
	"ScienceFiction", // 8 科幻未来 (site's own `Sci-Fi` slug 404s; ScienceFiction works)
	"Supernatural", // 9 奇异幻想
	"Healing", // 10 治愈温馨
	"Survival", // 11 末日生存
	"Other", // 12 其他分类
];

/// Mobile UA: the reader page serves its `<img data-src>` list only to
/// non-desktop-Chrome clients (a desktop Chrome UA gets a "请使用手机浏览器"
/// page with no images), so all requests carry a mobile UA.
const MOBILE_UA: &str = "Mozilla/5.0 (Linux; Android 13; Pixel 7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/125.0.0.0 Mobile Safari/537.36";

#[derive(Clone)]
pub enum Url {
	Filter {
		order: String,
		tagid: String,
		isfull: String,
		anime: String,
		rgroupid: String,
		sortid: String,
		update: String,
		quality: String,
		/// Named category slug from CATEGORY_SLUGS; empty = no 作品分类 selected.
		category: String,
		page: i32,
	},
	Search {
		query: String,
		page: i32,
	},
	Author {
		author: String,
	},
	Manga {
		id: String,
	},
	ChapterList {
		id: String,
	},
	Chapter {
		manga_id: String,
		chapter_id: String,
	},
}

impl Url {
	pub fn request(&self) -> Result<Request> {
		let url = self.to_string();
		let mut request = Request::get(url)?
			.header("Origin", BASE_URL)
			.header("User-Agent", MOBILE_UA)
			.header("Accept-Language", "zh-CN,zh;q=0.9")
			.header("Cookie", "night=0");

		// Add special referer for search requests
		if let Url::Search { query, .. } = self {
			let referer = format!("{}/search.html?searchkey={}", BASE_URL, query);
			request = request.header("Referer", &referer);
		}

		Ok(request)
	}

	pub fn from_query_or_filters(
		query: Option<&str>,
		page: i32,
		filters: &[FilterValue],
	) -> Result<Self> {
		if let Some(q) = query {
			return Ok(Self::Search {
				query: encode_uri(q),
				page,
			});
		}

		let mut tagid = "0".to_string();
		let mut sortid = "0".to_string();
		let mut rgroupid = "0".to_string();
		let mut order = "lastupdate".to_string();
		let mut anime = "0".to_string();
		let mut quality = "0".to_string();
		let mut isfull = "0".to_string();
		let mut update = "0".to_string();
		let mut category = String::new();

		for filter in filters {
			match filter {
				FilterValue::Text { id, value } => match id.as_str() {
					"author" => {
						// Special handling for author search: /author/{value}.html
						return Ok(Self::Author {
							author: encode_uri(value.clone()),
						});
					}
					_ => {
						// Title search
						return Ok(Self::Search {
							query: encode_uri(value.clone()),
							page,
						});
					}
				},
				FilterValue::Select { id, value } => match id.as_str() {
					"连载状态" => isfull = value.clone(),
					"是否动画" => anime = value.clone(),
					"是否轻改" => quality = value.clone(),
					"文库地区" => rgroupid = value.clone(),
					"更新时间" => update = value.clone(),
					"作品分类" => {
						sortid = value.clone();
						// Map to the named category URL; the numeric sortid only
						// covers a subset of categories (科幻未来/奇异幻想 have no
						// stable numeric id), so use the site's canonical slug.
						if let Ok(n) = value.parse::<usize>()
							&& (1..CATEGORY_SLUGS.len()).contains(&n)
						{
							category = CATEGORY_SLUGS[n].to_string();
						}
					}
					"作品主题" => {
						// Single-select now (multi-select values 404 on the site);
						// value is the selected tag id.
						tagid = value.clone();
					}
					"genre" => {
						let genre_options = [
							"奇幻",
							"冒險",
							"異世界",
							"龍傲天",
							"魔法",
							"仙俠",
							"戰爭",
							"熱血",
							"戰鬥",
							"競技",
							"懸疑",
							"驚悚",
							"獵奇",
							"神鬼",
							"偵探",
							"校園",
							"日常",
							"JK",
							"JC",
							"青梅竹馬",
							"妹妹",
							"大小姐",
							"女兒",
							"戀愛",
							"耽美",
							"百合",
							"NTR",
							"後宮",
							"職場",
							"經營",
							"犯罪",
							"旅行",
							"群像",
							"女性視角",
							"歷史",
							"武俠",
							"東方",
							"勵志",
							"宅系",
							"科幻",
							"機戰",
							"遊戲",
							"異能",
							"腦洞",
							"病嬌",
							"人外",
							"復仇",
							"鬥智",
							"惡役",
							"間諜",
							"治癒",
							"歡樂",
							"萌系",
							"末日",
							"大逃殺",
							"音樂",
							"美食",
							"性轉",
							"偽娘",
							"穿越",
							"童話",
							"轉生",
							"黑暗",
							"溫馨",
							"超自然",
						];

						if let Some(index) = genre_options
							.iter()
							.position(|&option| option == value.as_str())
						{
							tagid = (index + 1).to_string();
						}
					}
					_ => {}
				},
				FilterValue::Sort { id, index, .. } if id.as_str() == "排序" => {
					let orders = [
						"lastupdate",
						"postdate",
						"weekvisit",
						"monthvisit",
						"weekvote",
						"monthvote",
						"weekflower",
						"monthflower",
						"words",
						"goodnum",
					];
					if let Some(o) = orders.get(*index as usize) {
						order = o.to_string();
					}
				}
				_ => {}
			}
		}

		Ok(Self::Filter {
			order,
			tagid,
			isfull,
			anime,
			rgroupid,
			sortid,
			update,
			quality,
			category,
			page,
		})
	}

	pub fn manga(id: String) -> Self {
		Self::Manga { id }
	}

	pub fn chapter_list(id: String) -> Self {
		Self::ChapterList { id }
	}

	pub fn chapter(manga_id: String, chapter_id: String) -> Self {
		Self::Chapter {
			manga_id,
			chapter_id,
		}
	}
}

impl Display for Url {
	fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
		match self {
			Url::Filter {
				order,
				tagid,
				isfull,
				anime,
				rgroupid,
				sortid,
				update,
				quality,
				category,
				page,
			} if !category.is_empty() => {
				// Named category page (standalone, like the site's own nav):
				// /filter/{slug}/{page}.html
				write!(f, "{}/filter/{}/{}.html", BASE_URL, category, page)
			}
			Url::Filter {
				order,
				tagid,
				isfull,
				anime,
				rgroupid,
				sortid,
				update,
				quality,
				category: _,
				page,
			} => {
				// The site's /filter/ URL has 12 segments: the 9 real params
				// (order/tagid/isfull/anime/rgroupid/sortid/update/quality/page)
				// followed by three trailing 0s. Building 10 segments (as the
				// upstream source did) returns 404 since the site redesign.
				write!(
					f,
					"{}/filter/{}_{}_{}_{}_{}_{}_{}_{}_{}_0_0_0.html",
					BASE_URL, order, tagid, isfull, anime, rgroupid, sortid, update, quality, page
				)
			}
			Url::Search { query, page } => {
				if *page == 1 {
					write!(f, "{}/search.html?searchkey={}", BASE_URL, query)
				} else {
					write!(f, "{}/search/{}_{}.html", BASE_URL, query, page)
				}
			}
			Url::Author { author } => {
				write!(f, "{}/author/{}.html", BASE_URL, author)
			}
			Url::Manga { id } => {
				write!(f, "{}/detail/{}.html", BASE_URL, id)
			}
			Url::ChapterList { id } => {
				write!(f, "{}/read/{}/catalog", BASE_URL, id)
			}
			Url::Chapter {
				manga_id,
				chapter_id,
			} => {
				write!(f, "{}/read/{}/{}.html", BASE_URL, manga_id, chapter_id)
			}
		}
	}
}
