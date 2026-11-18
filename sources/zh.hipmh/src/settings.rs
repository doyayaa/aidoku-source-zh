use aidoku::imports::defaults::defaults_get;

// Segment settings store the selected index as Int (1 = 列表), not a bool.
pub fn get_list_viewer() -> bool {
	defaults_get::<i32>("isListView").unwrap_or(0) == 1
}