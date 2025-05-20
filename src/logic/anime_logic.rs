use crate::{Anime, AnimeData, AppWindow, DayAnime};
use crate::logic::APP_PATH;
use scraper::{Html, Selector};
use slint::{
    ComponentHandle, Image, Model, Rgba8Pixel, SharedPixelBuffer, Weak,
    invoke_from_event_loop,
};
use std::{io::Write, path::PathBuf, rc::Rc};

const BASE_URL: &str = "https://yuc.wiki/202501";

pub fn anime_logic(app: Weak<AppWindow>) {
    std::thread::spawn(move || {
        let list = parse_html(BASE_URL);
        invoke_from_event_loop(move || {
            let app = app.unwrap();
            let mut week_anime_list = app
                .global::<AnimeData>()
                .get_week_anime_list()
                .iter()
                .collect::<Vec<DayAnime>>();

            for (i, name) in list.into_iter().enumerate() {
                let list = name
                    .into_iter()
                    .map(|(n, img)| Anime {
                        name: n.into(),
                        cover: Image::from_rgba8(img),
                    })
                    .collect::<Vec<Anime>>();
                week_anime_list[i].anime_list = Rc::new(slint::VecModel::from(list)).into();
            }

            app.global::<AnimeData>()
                .set_week_anime_list(Rc::new(slint::VecModel::from(week_anime_list)).into());
        })
        .unwrap();
    });
}

fn parse_html(url: &str) -> Vec<Vec<(String, SharedPixelBuffer<Rgba8Pixel>)>> {
    let anime_data_path = APP_PATH.join("data").join("anime.json");
    if anime_data_path.exists() {
        let list: Vec<Vec<String>> =
            serde_json::from_reader(std::fs::File::open(&anime_data_path).unwrap()).unwrap();
        let week_anime_list = list
            .into_iter()
            .map(|n| {
                n.into_iter()
                    .map(|n| {
                        let path = APP_PATH.join("covers").join(&n).with_extension("jpg");
                        (n, load_img_from_path(path))
                    })
                    .collect::<Vec<(String, SharedPixelBuffer<Rgba8Pixel>)>>()
            })
            .collect();
        return week_anime_list;
    }
    let mut name_list = vec![];
    let mut week_anime_list = vec![];
    let response = reqwest::blocking::get(url).unwrap();
    let html = response.text().unwrap();
    let document = Html::parse_document(&html);
    let selector = Selector::parse("div.post-body>div").unwrap();

    document
        .select(&selector)
        .take(20)
        .skip(1)
        .step_by(3)
        .for_each(|e| {
            let name_selector = Selector::parse("tr:nth-child(1)>td").unwrap();
            let cover_selector = Selector::parse("div.div_date img").unwrap();
            let names = e
                .select(&name_selector)
                .map(|ce| {
                    let r_str = ce.text().collect::<String>();
                    get_valid_filename(&r_str)
                })
                .collect::<Vec<String>>();
            let covers = e
                .select(&cover_selector)
                .map(|ce| ce.value().attr("data-src").unwrap_or("").to_string())
                .collect::<Vec<String>>();
            let anime_list = names
                .iter()
                .zip(covers.into_iter())
                .map(|(n, c)| {
                    let cover_path = get_cover(&c, &n);
                    let img = load_img_from_path(cover_path);
                    (n.to_owned(), img)
                })
                .collect::<Vec<(String, SharedPixelBuffer<Rgba8Pixel>)>>();
            name_list.push(names);
            week_anime_list.push(anime_list);
        });
    serde_json::to_writer_pretty(std::fs::File::create(&anime_data_path).unwrap(), &name_list)
        .unwrap();
    week_anime_list
}

fn get_cover(cover: &str, name: &str) -> PathBuf {
    let save_path = APP_PATH.join("covers");
    let path = save_path.join(name).with_extension("jpg");
    if path.exists() {
        return path;
    }
    let response = reqwest::blocking::get(cover).unwrap();
    let bytes = response.bytes().unwrap();
    let mut file = std::fs::File::create(&path).unwrap();
    file.write_all(&bytes).unwrap();
    path
}

//确保字符串符合文件名的要求，如果不符合要求，则加以修改
fn get_valid_filename(name: &str) -> String {
    let invalid_chars = ['/', '\\', ':', '*', '?', '"', '<', '>', '|'];
    let mut valid_name = String::new();
    for c in name.chars() {
        if invalid_chars.contains(&c) {
            valid_name.push('_');
        } else {
            valid_name.push(c);
        }
    }
    valid_name
}

fn load_img_from_path(path: PathBuf) -> SharedPixelBuffer<Rgba8Pixel> {
    let img = image::open(&path).unwrap().into_rgba8();
    SharedPixelBuffer::<Rgba8Pixel>::clone_from_slice(img.as_raw(), img.width(), img.height())
}
