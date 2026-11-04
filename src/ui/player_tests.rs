use super::*;
use iced_graphics::core::{Rectangle, widget::{Id, Operation}};
use iced_runtime::user_interface::{Cache, UserInterface};

#[test]
fn pitch_controls_fit_inside_the_decks_with_bottom_padding() {
    struct Bounds(Vec<(Id,Rectangle)>);
    impl Operation for Bounds {
        fn traverse(&mut self, children: &mut dyn FnMut(&mut dyn Operation)) { children(self); }
        fn container(&mut self, id: Option<&Id>, bounds: Rectangle) { if let Some(id) = id { self.0.push((id.clone(),bounds)); } }
    }
    for width in [1100.0, 1920.0] {
        let temp = tempfile::tempdir().unwrap();
        let mut app = App::load(temp.path().join("library.json"),None).unwrap();
        app.preview.visible=true;
        let mut renderer=super::rendering::new_renderer();
        let mut ui=UserInterface::build(app.view(),iced::Size::new(width,900.0),Cache::new(),&mut renderer);
        let mut bounds=Bounds(Vec::new()); ui.operate(&renderer,&mut bounds);
        let get=|name| bounds.0.iter().find(|(id,_)|*id==Id::new(name)).unwrap().1;
        for (panel,pitch) in [("deck-a-panel","pitch-controls-A"),("deck-b-panel","pitch-controls-B")] {
            let (panel,pitch)=(get(panel),get(pitch));
            assert!(pitch.height >= 24.0, "Pitch/reset controls lose their full hit target: {pitch:?}");
            assert!(pitch.y+pitch.height <= panel.y+panel.height-10.0,"Pitch/reset spills outside its panel: {panel:?} / {pitch:?}");
        }
    }
}
