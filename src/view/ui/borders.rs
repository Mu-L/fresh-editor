use ratatui::symbols::border;

pub fn get_border_set(advanced_unicode_borders: bool) -> border::Set {
    if advanced_unicode_borders {
        border::Set {
            top_left: "🭽",
            top_right: "🭾",
            bottom_left: "🭼",
            bottom_right: "🭿",
            horizontal_top: "─",
            horizontal_bottom: "─",
            vertical_left: "│",
            vertical_right: "│",
        }
    } else {
        border::PLAIN
    }
}
