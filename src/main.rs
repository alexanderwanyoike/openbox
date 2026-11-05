use clap::Parser;
use openbox::ui::App;
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(
    version,
    about = "Organise DJ playlists and manage Rekordbox USB libraries"
)]
struct Args {
    #[arg(long, help = "Open the separate demo library")]
    demo: bool,

    #[arg(long, value_name = "FILE", help = "Local library JSON file")]
    library: Option<PathBuf>,

    #[arg(long, value_name = "DIRECTORY", help = "Root of a mounted USB")]
    usb: Option<PathBuf>,
}

impl Args {
    fn library_path(&self) -> PathBuf {
        self.library.clone().unwrap_or_else(|| {
            PathBuf::from(if self.demo {
                ".prototype/demo-library.json"
            } else {
                ".prototype/library.json"
            })
        })
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let app = App::load(args.library_path(), args.usb).map_err(std::io::Error::other)?;
    iced::application(
        move || {
            let task = app.startup();
            (app.clone(), task)
        },
        App::update,
        App::view,
    )
    .font(include_bytes!("../resources/fonts/Manrope.ttf").as_slice())
    .font(include_bytes!("../resources/fonts/Manrope-SemiBold.ttf").as_slice())
    .font(include_bytes!("../resources/fonts/Manrope-Bold.ttf").as_slice())
    .default_font(iced::Font::with_name("Manrope"))
    .title("Openbox")
    .theme(App::theme)
    .subscription(App::subscription)
    .window(iced::window::Settings {
        size: iced::Size::new(1440.0, 900.0),
        min_size: Some(iced::Size::new(1000.0, 720.0)),
        decorations: false,
        #[cfg(target_os = "linux")]
        platform_specific: iced::window::settings::PlatformSpecific {
            application_id: openbox::branding::APPLICATION_ID.into(),
            ..Default::default()
        },
        icon: Some(openbox::branding::window_icon().map_err(std::io::Error::other)?),
        ..Default::default()
    })
    .run()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn startup_flags_accept_paths_and_reject_missing_or_unknown_arguments() {
        let args = Args::try_parse_from([
            "openbox",
            "--demo",
            "--library",
            "/music/my library.json",
            "--usb",
            "/media/DJ USB",
        ])
        .unwrap();
        assert!(args.demo);
        assert_eq!(args.library_path(), PathBuf::from("/music/my library.json"));
        assert_eq!(args.usb, Some(PathBuf::from("/media/DJ USB")));
        assert!(Args::try_parse_from(["openbox", "--usb"]).is_err());
        assert!(Args::try_parse_from(["openbox", "--unknown"]).is_err());
        assert_eq!(
            Args::try_parse_from(["openbox", "--demo"])
                .unwrap()
                .library_path(),
            PathBuf::from(".prototype/demo-library.json")
        );
        assert_eq!(
            Args::try_parse_from(["openbox"]).unwrap().library_path(),
            PathBuf::from(".prototype/library.json")
        );
    }
}
