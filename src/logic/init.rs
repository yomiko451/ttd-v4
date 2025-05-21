use crate::AppWindow;
use slint::Weak;
use std::path::PathBuf;
use std::sync::LazyLock;

pub const APP_PATH: LazyLock<PathBuf> = LazyLock::new(|| {
    let path = std::env::var("APPDATA").unwrap();
    let app_path = PathBuf::from(path).join(".ttd_v4");
    if !app_path.exists() {
        std::fs::create_dir(&app_path).unwrap();
    }
    app_path
});

pub fn init(app: Weak<AppWindow>) {
    check_data_dir();
    let anime_schedule = crate::logic::init_anime_schedule(app.clone());
    crate::logic::get_anime(app, anime_schedule);
}

fn check_data_dir() {
    let anime_path = APP_PATH.join("covers");
    if !anime_path.exists() {
        std::fs::create_dir(&anime_path).unwrap();
    }
    let data_path = APP_PATH.join("data");
    if !data_path.exists() {
        std::fs::create_dir(&data_path).unwrap();
    }
}
