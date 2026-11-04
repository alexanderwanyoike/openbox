use super::*;
use iced_graphics::core::{
    Rectangle,
    widget::{Id, Operation},
};
use iced_runtime::user_interface::{Cache, UserInterface};

#[test]
fn chrome_is_separate_from_workspace_actions_and_collection_rows_are_inset() {
    struct Bounds(Vec<(Id, Rectangle)>);
    impl Operation for Bounds {
        fn traverse(&mut self, children: &mut dyn FnMut(&mut dyn Operation)) {
            children(self);
        }
        fn scrollable(
            &mut self,
            id: Option<&Id>,
            bounds: Rectangle,
            _: Rectangle,
            _: iced::Vector,
            _: &mut dyn iced_graphics::core::widget::operation::Scrollable,
        ) {
            if let Some(id) = id {
                self.0.push((id.clone(), bounds));
            }
        }
        fn container(&mut self, id: Option<&Id>, bounds: Rectangle) {
            if let Some(id) = id {
                self.0.push((id.clone(), bounds));
            }
        }
    }
    for width in [1100.0, 1440.0] {
        let temp = tempfile::tempdir().unwrap();
        let mut app = App::load(temp.path().join("library.json"), None).unwrap();
        app.preview.visible = true;
        let mut renderer = super::rendering::new_renderer();
        let mut ui = UserInterface::build(
            app.view(),
            iced::Size::new(width, 900.0),
            Cache::new(),
            &mut renderer,
        );
        let mut found = Bounds(Vec::new());
        ui.operate(&renderer, &mut found);
        let bounds = |name| {
            found
                .0
                .iter()
                .find(|(id, _)| *id == Id::new(name))
                .map(|(_, r)| *r)
                .unwrap_or_else(|| panic!("Missing visible {name}"))
        };
        let chrome = bounds("application-chrome");
        let toolbar = bounds("workspace-toolbar");
        assert_eq!(chrome.height, 48.0);
        assert_eq!(toolbar.y, chrome.y + chrome.height);
        assert_eq!(toolbar.height, 48.0);
        let tree = bounds("playlist-tree");
        assert!(
            tree.height >= 260.0,
            "Creation controls must not crowd out device navigation: {tree:?}"
        );
        let sidebar = bounds("collection-panel");
        let collection = bounds("collection-row");
        assert_eq!(sidebar.width, 212.0);
        assert!(collection.x >= sidebar.x + 9.0);
        assert!(collection.x + collection.width <= sidebar.x + sidebar.width - 9.0);
        assert!(
            sidebar.height >= 260.0,
            "The tree must remain usable with decks visible"
        );
    }
}
