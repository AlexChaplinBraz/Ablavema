//#![allow(dead_code, unused_imports, unused_variables)]
use iced::{button, checkbox, container, pick_list, progress_bar, radio, rule, Color};
use serde::{Deserialize, Serialize};

/// Creates a const Color. Takes values from 0 to 255.
/// First argument is the const name.
/// Then Red, Green, Blue, and optionally Alpha.
macro_rules! const_color {
    ($const_name:ident, $red:expr, $green:expr, $blue:expr) => {
        pub const $const_name: Color = Color::from_rgb(
            $red as f32 / 255.0,
            $green as f32 / 255.0,
            $blue as f32 / 255.0,
        );
    };
    ($const_name:ident, $red:expr, $green:expr, $blue:expr, $alpha:expr) => {
        pub const $const_name: Color = Color::from_rgba(
            $red as f32 / 255.0,
            $green as f32 / 255.0,
            $blue as f32 / 255.0,
            $alpha as f32 / 255.0,
        );
    };
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum Theme {
    Light,
    Dark,
}

impl Theme {
    pub const ALL: [Theme; 2] = [Theme::Light, Theme::Dark];

    pub fn tab_button(&self) -> Box<dyn button::StyleSheet> {
        match self {
            Theme::Light => light::ButtonTab.into(),
            Theme::Dark => dark::ButtonTab.into(),
        }
    }

    pub fn tab_container(&self) -> Box<dyn container::StyleSheet> {
        match self {
            Theme::Light => light::ContainerTab.into(),
            Theme::Dark => dark::ContainerTab.into(),
        }
    }

    pub fn info_container(&self) -> Box<dyn container::StyleSheet> {
        match self {
            Theme::Light => light::ContainerInfo.into(),
            Theme::Dark => dark::ContainerInfo.into(),
        }
    }

    pub fn sidebar_container(&self) -> Box<dyn container::StyleSheet> {
        match self {
            Theme::Light => light::ContainerSidebar.into(),
            Theme::Dark => dark::ContainerSidebar.into(),
        }
    }

    pub fn odd_container(&self) -> Box<dyn container::StyleSheet> {
        match self {
            Theme::Light => light::ContainerOdd.into(),
            Theme::Dark => dark::ContainerOdd.into(),
        }
    }

    pub fn even_container(&self) -> Box<dyn container::StyleSheet> {
        match self {
            Theme::Light => light::ContainerEven.into(),
            Theme::Dark => dark::ContainerEven.into(),
        }
    }

    pub fn status_container(&self) -> Box<dyn container::StyleSheet> {
        match self {
            Theme::Light => light::ContainerStatus.into(),
            Theme::Dark => dark::ContainerStatus.into(),
        }
    }

    pub fn highlight_text(&self) -> Color {
        match self {
            Theme::Light => light::ACTIVE_TEXT,
            Theme::Dark => dark::ACTIVE_TEXT,
        }
    }
}

impl Default for Theme {
    fn default() -> Theme {
        Theme::Dark
    }
}

impl From<Theme> for Box<dyn container::StyleSheet> {
    fn from(theme: Theme) -> Self {
        match theme {
            Theme::Light => light::Container.into(),
            Theme::Dark => dark::Container.into(),
        }
    }
}

impl From<Theme> for Box<dyn radio::StyleSheet> {
    fn from(theme: Theme) -> Self {
        match theme {
            Theme::Light => light::Radio.into(),
            Theme::Dark => dark::Radio.into(),
        }
    }
}

impl From<Theme> for Box<dyn button::StyleSheet> {
    fn from(theme: Theme) -> Self {
        match theme {
            Theme::Light => light::Button.into(),
            Theme::Dark => dark::Button.into(),
        }
    }
}

impl From<Theme> for Box<dyn progress_bar::StyleSheet> {
    fn from(theme: Theme) -> Self {
        match theme {
            Theme::Light => light::ProgressBar.into(),
            Theme::Dark => dark::ProgressBar.into(),
        }
    }
}

impl From<Theme> for Box<dyn checkbox::StyleSheet> {
    fn from(theme: Theme) -> Self {
        match theme {
            Theme::Light => light::Checkbox.into(),
            Theme::Dark => dark::Checkbox.into(),
        }
    }
}

impl From<Theme> for Box<dyn pick_list::StyleSheet> {
    fn from(theme: Theme) -> Self {
        match theme {
            Theme::Light => light::PickList.into(),
            Theme::Dark => dark::PickList.into(),
        }
    }
}

impl From<Theme> for Box<dyn rule::StyleSheet> {
    fn from(theme: Theme) -> Self {
        match theme {
            Theme::Light => light::Rule.into(),
            Theme::Dark => dark::Rule.into(),
        }
    }
}

mod light {
    use iced::{button, checkbox, container, pick_list, progress_bar, radio, rule, Color, Vector};

    const_color!(ACTIVE_TAB, 190, 190, 190);
    const_color!(HOVERED_TAB, 142, 142, 142);
    const_color!(INACTIVE_TAB, 129, 129, 129);
    const_color!(ACTIVE, 86, 128, 194);
    const_color!(HOVERED, 241, 241, 241);
    const_color!(INACTIVE, 219, 219, 219);
    const_color!(CONTAINER_BACKGROUND, 166, 166, 166);
    const_color!(TAB_BACKGROUND, 179, 179, 179);
    const_color!(INFO_BACKGROUND, 169, 169, 169);
    const_color!(SIDEBAR_BACKGROUND, 163, 163, 163);
    const_color!(PICK_LIST_BACKGROUND, 213, 213, 213);
    const_color!(ODD_BACKGROUND, 153, 153, 153);
    const_color!(EVEN_BACKGROUND, 159, 159, 159);
    const_color!(STATUS_BACKGROUND, 255, 0, 0);
    const_color!(TEXT, 26, 26, 26);
    const_color!(ACTIVE_TEXT, 0, 0, 0);

    pub struct Button;
    impl button::StyleSheet for Button {
        fn active(&self) -> button::Style {
            button::Style {
                shadow_offset: Vector::default(),
                background: INACTIVE.into(),
                border_radius: 5.0,
                border_width: 0.0,
                border_color: Color::TRANSPARENT,
                text_color: TEXT,
            }
        }

        fn hovered(&self) -> button::Style {
            button::Style {
                background: HOVERED.into(),
                text_color: ACTIVE_TEXT,
                ..self.active()
            }
        }

        fn pressed(&self) -> button::Style {
            button::Style {
                background: ACTIVE.into(),
                ..self.hovered()
            }
        }
    }

    pub struct ButtonTab;
    impl button::StyleSheet for ButtonTab {
        fn active(&self) -> button::Style {
            button::Style {
                shadow_offset: Vector::default(),
                background: INACTIVE_TAB.into(),
                border_radius: 2.0,
                border_width: 0.0,
                border_color: Color::TRANSPARENT,
                text_color: TEXT,
            }
        }

        fn hovered(&self) -> button::Style {
            button::Style {
                background: HOVERED_TAB.into(),
                text_color: ACTIVE_TEXT,
                ..self.active()
            }
        }

        fn pressed(&self) -> button::Style {
            self.hovered()
        }

        fn disabled(&self) -> button::Style {
            button::Style {
                background: ACTIVE_TAB.into(),
                text_color: ACTIVE_TEXT,
                ..self.active()
            }
        }
    }

    pub struct Checkbox;
    impl checkbox::StyleSheet for Checkbox {
        fn active(&self, is_checked: bool) -> checkbox::Style {
            checkbox::Style {
                background: if is_checked { ACTIVE } else { INACTIVE }.into(),
                checkmark_color: TEXT,
                border_radius: 5.0,
                border_width: 0.0,
                border_color: Color::TRANSPARENT,
            }
        }

        fn hovered(&self, is_checked: bool) -> checkbox::Style {
            checkbox::Style {
                background: if is_checked { ACTIVE } else { HOVERED }.into(),
                checkmark_color: ACTIVE_TEXT,
                ..self.active(is_checked)
            }
        }
    }

    pub struct Container;
    impl container::StyleSheet for Container {
        fn style(&self) -> container::Style {
            container::Style {
                background: CONTAINER_BACKGROUND.into(),
                text_color: TEXT.into(),
                border_radius: 0.0,
                border_width: 0.0,
                border_color: Color::TRANSPARENT,
            }
        }
    }

    pub struct ContainerTab;
    impl container::StyleSheet for ContainerTab {
        fn style(&self) -> container::Style {
            container::Style {
                background: TAB_BACKGROUND.into(),
                ..Container.style()
            }
        }
    }

    pub struct ContainerInfo;
    impl container::StyleSheet for ContainerInfo {
        fn style(&self) -> container::Style {
            container::Style {
                background: INFO_BACKGROUND.into(),
                ..Container.style()
            }
        }
    }

    pub struct ContainerSidebar;
    impl container::StyleSheet for ContainerSidebar {
        fn style(&self) -> container::Style {
            container::Style {
                background: SIDEBAR_BACKGROUND.into(),
                ..Container.style()
            }
        }
    }

    pub struct ContainerOdd;
    impl container::StyleSheet for ContainerOdd {
        fn style(&self) -> container::Style {
            container::Style {
                background: ODD_BACKGROUND.into(),
                ..Container.style()
            }
        }
    }

    pub struct ContainerEven;
    impl container::StyleSheet for ContainerEven {
        fn style(&self) -> container::Style {
            container::Style {
                background: EVEN_BACKGROUND.into(),
                ..Container.style()
            }
        }
    }

    pub struct ContainerStatus;
    impl container::StyleSheet for ContainerStatus {
        fn style(&self) -> container::Style {
            container::Style {
                background: STATUS_BACKGROUND.into(),
                ..Container.style()
            }
        }
    }

    pub struct PickList;
    impl pick_list::StyleSheet for PickList {
        fn menu(&self) -> pick_list::Menu {
            pick_list::Menu {
                text_color: TEXT,
                background: PICK_LIST_BACKGROUND.into(),
                border_width: 0.0,
                border_color: Color::TRANSPARENT,
                selected_text_color: ACTIVE_TEXT,
                selected_background: HOVERED.into(),
            }
        }

        fn active(&self) -> pick_list::Style {
            pick_list::Style {
                text_color: ACTIVE_TEXT,
                background: PICK_LIST_BACKGROUND.into(),
                border_radius: 5.0,
                border_width: 0.0,
                border_color: Color::TRANSPARENT,
                icon_size: 0.4,
            }
        }

        fn hovered(&self) -> pick_list::Style {
            pick_list::Style {
                background: HOVERED.into(),
                ..self.active()
            }
        }
    }

    pub struct ProgressBar;
    impl progress_bar::StyleSheet for ProgressBar {
        fn style(&self) -> progress_bar::Style {
            progress_bar::Style {
                background: INACTIVE.into(),
                bar: ACTIVE.into(),
                border_radius: 5.0,
            }
        }
    }

    pub struct Radio;
    impl radio::StyleSheet for Radio {
        fn active(&self) -> radio::Style {
            radio::Style {
                background: INACTIVE.into(),
                dot_color: ACTIVE,
                border_width: 0.0,
                border_color: Color::TRANSPARENT,
            }
        }

        fn hovered(&self) -> radio::Style {
            radio::Style {
                background: HOVERED.into(),
                ..self.active()
            }
        }
    }

    pub struct Rule;
    impl rule::StyleSheet for Rule {
        fn style(&self) -> rule::Style {
            rule::Style {
                color: INACTIVE,
                width: 2,
                radius: 1.0,
                fill_mode: rule::FillMode::Full,
            }
        }
    }
}

mod dark {
    use iced::{button, checkbox, container, pick_list, progress_bar, radio, rule, Color, Vector};

    const_color!(ACTIVE_TAB, 66, 66, 66);
    const_color!(HOVERED_TAB, 52, 52, 52);
    const_color!(INACTIVE_TAB, 43, 43, 43);
    const_color!(ACTIVE, 83, 121, 180);
    const_color!(HOVERED, 106, 106, 106);
    const_color!(INACTIVE, 88, 88, 88);
    const_color!(CONTAINER_BACKGROUND, 56, 56, 56);
    const_color!(TAB_BACKGROUND, 35, 35, 35);
    const_color!(INFO_BACKGROUND, 59, 59, 59);
    const_color!(SIDEBAR_BACKGROUND, 51, 51, 51);
    const_color!(PICK_LIST_BACKGROUND, 44, 44, 44);
    const_color!(ODD_BACKGROUND, 40, 40, 40);
    const_color!(EVEN_BACKGROUND, 45, 45, 45);
    const_color!(STATUS_BACKGROUND, 255, 0, 0);
    const_color!(TEXT, 217, 217, 217);
    const_color!(ACTIVE_TEXT, 255, 255, 255);

    pub struct Button;
    impl button::StyleSheet for Button {
        fn active(&self) -> button::Style {
            button::Style {
                shadow_offset: Vector::default(),
                background: INACTIVE.into(),
                border_radius: 5.0,
                border_width: 0.0,
                border_color: Color::TRANSPARENT,
                text_color: TEXT,
            }
        }

        fn hovered(&self) -> button::Style {
            button::Style {
                background: HOVERED.into(),
                text_color: ACTIVE_TEXT,
                ..self.active()
            }
        }

        fn pressed(&self) -> button::Style {
            button::Style {
                background: ACTIVE.into(),
                ..self.hovered()
            }
        }
    }

    pub struct ButtonTab;
    impl button::StyleSheet for ButtonTab {
        fn active(&self) -> button::Style {
            button::Style {
                shadow_offset: Vector::default(),
                background: INACTIVE_TAB.into(),
                border_radius: 2.0,
                border_width: 0.0,
                border_color: Color::TRANSPARENT,
                text_color: TEXT,
            }
        }

        fn hovered(&self) -> button::Style {
            button::Style {
                background: HOVERED_TAB.into(),
                text_color: ACTIVE_TEXT,
                ..self.active()
            }
        }

        fn pressed(&self) -> button::Style {
            self.hovered()
        }

        fn disabled(&self) -> button::Style {
            button::Style {
                background: ACTIVE_TAB.into(),
                text_color: ACTIVE_TEXT,
                ..self.active()
            }
        }
    }

    pub struct Checkbox;
    impl checkbox::StyleSheet for Checkbox {
        fn active(&self, is_checked: bool) -> checkbox::Style {
            checkbox::Style {
                background: if is_checked { ACTIVE } else { INACTIVE }.into(),
                checkmark_color: TEXT,
                border_radius: 5.0,
                border_width: 0.0,
                border_color: Color::TRANSPARENT,
            }
        }

        fn hovered(&self, is_checked: bool) -> checkbox::Style {
            checkbox::Style {
                background: if is_checked { ACTIVE } else { HOVERED }.into(),
                checkmark_color: ACTIVE_TEXT,
                ..self.active(is_checked)
            }
        }
    }

    pub struct Container;
    impl container::StyleSheet for Container {
        fn style(&self) -> container::Style {
            container::Style {
                background: CONTAINER_BACKGROUND.into(),
                text_color: TEXT.into(),
                border_radius: 0.0,
                border_width: 0.0,
                border_color: Color::TRANSPARENT,
            }
        }
    }

    pub struct ContainerTab;
    impl container::StyleSheet for ContainerTab {
        fn style(&self) -> container::Style {
            container::Style {
                background: TAB_BACKGROUND.into(),
                ..Container.style()
            }
        }
    }

    pub struct ContainerInfo;
    impl container::StyleSheet for ContainerInfo {
        fn style(&self) -> container::Style {
            container::Style {
                background: INFO_BACKGROUND.into(),
                ..Container.style()
            }
        }
    }

    pub struct ContainerSidebar;
    impl container::StyleSheet for ContainerSidebar {
        fn style(&self) -> container::Style {
            container::Style {
                background: SIDEBAR_BACKGROUND.into(),
                ..Container.style()
            }
        }
    }

    pub struct ContainerOdd;
    impl container::StyleSheet for ContainerOdd {
        fn style(&self) -> container::Style {
            container::Style {
                background: ODD_BACKGROUND.into(),
                ..Container.style()
            }
        }
    }

    pub struct ContainerEven;
    impl container::StyleSheet for ContainerEven {
        fn style(&self) -> container::Style {
            container::Style {
                background: EVEN_BACKGROUND.into(),
                ..Container.style()
            }
        }
    }

    pub struct ContainerStatus;
    impl container::StyleSheet for ContainerStatus {
        fn style(&self) -> container::Style {
            container::Style {
                background: STATUS_BACKGROUND.into(),
                ..Container.style()
            }
        }
    }

    pub struct PickList;
    impl pick_list::StyleSheet for PickList {
        fn menu(&self) -> pick_list::Menu {
            pick_list::Menu {
                text_color: TEXT,
                background: PICK_LIST_BACKGROUND.into(),
                border_width: 0.0,
                border_color: Color::TRANSPARENT,
                selected_text_color: ACTIVE_TEXT,
                selected_background: HOVERED.into(),
            }
        }

        fn active(&self) -> pick_list::Style {
            pick_list::Style {
                text_color: ACTIVE_TEXT,
                background: PICK_LIST_BACKGROUND.into(),
                border_radius: 5.0,
                border_width: 0.0,
                border_color: Color::TRANSPARENT,
                icon_size: 0.4,
            }
        }

        fn hovered(&self) -> pick_list::Style {
            pick_list::Style {
                background: HOVERED.into(),
                ..self.active()
            }
        }
    }

    pub struct ProgressBar;
    impl progress_bar::StyleSheet for ProgressBar {
        fn style(&self) -> progress_bar::Style {
            progress_bar::Style {
                background: INACTIVE.into(),
                bar: ACTIVE.into(),
                border_radius: 5.0,
            }
        }
    }

    pub struct Radio;
    impl radio::StyleSheet for Radio {
        fn active(&self) -> radio::Style {
            radio::Style {
                background: INACTIVE.into(),
                dot_color: ACTIVE,
                border_width: 0.0,
                border_color: Color::TRANSPARENT,
            }
        }

        fn hovered(&self) -> radio::Style {
            radio::Style {
                background: HOVERED.into(),
                ..self.active()
            }
        }
    }

    pub struct Rule;
    impl rule::StyleSheet for Rule {
        fn style(&self) -> rule::Style {
            rule::Style {
                color: INACTIVE,
                width: 2,
                radius: 1.0,
                fill_mode: rule::FillMode::Full,
            }
        }
    }
}
