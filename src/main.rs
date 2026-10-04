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
    iced::application(move || app.clone(), App::update, App::view)
        .title("Openbox")
        .theme(App::theme)
        .subscription(App::subscription)
        .window_size(iced::Size::new(1440.0, 900.0))
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
