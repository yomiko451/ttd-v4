use crate::{logic::APP_PATH, logic::CURRENT_DATE};
use crate::{Anime, AnimeData, AppWindow, logic::SlintDate, DayAnime};
use chrono::{Datelike, Local, NaiveDate};
use reqwest::Client;
use scraper::{Html, Selector};
use slint::{
    ComponentHandle, Image, Model, Rgba8Pixel, SharedPixelBuffer, Weak, invoke_from_event_loop,
};
use std::{io::Write, rc::Rc, sync::LazyLock};
use tokio::runtime::Runtime;

const BASE_URL: &str = "https://yuc.wiki/";


pub fn set_anime_logic(app_weak: Weak<AppWindow>) {
    let app = app_weak.unwrap();
    let anime_data = app.global::<AnimeData>();
    let weak = app_weak.clone();
    //TODO: 
}

pub fn get_anime(app_weak: Weak<AppWindow>, anime_schedule: SlintDate) {
    let suffix = get_suffix(anime_schedule);
    std::thread::spawn(move || {
        let list = parse_html(suffix);
        invoke_from_event_loop(move || {
            let app = app_weak.unwrap();
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
                        cover: Image::from_rgba8(img.unwrap()), //TODO: 这里需要处理图片加载失败的情况
                    })
                    .collect::<Vec<Anime>>();
                week_anime_list[i].anime_list = Rc::new(slint::VecModel::from(list)).into();
            }

            let anime_data = app.global::<AnimeData>();
                anime_data.set_week_anime_list(Rc::new(slint::VecModel::from(week_anime_list)).into());
                anime_data.set_is_loading(false);
        })
        .unwrap();
    });
}

fn parse_html(suffix: String) -> Vec<Vec<(String, Option<SharedPixelBuffer<Rgba8Pixel>>)>> {
    let anime_data_path = APP_PATH.join("data").join(format!("{}.json", suffix));
    let url = format!("{}{}", BASE_URL, suffix);
    if anime_data_path.exists() {
        let list: Vec<Vec<String>> =
            serde_json::from_reader(std::fs::File::open(&anime_data_path).unwrap()).unwrap();
        let week_anime_list = list
            .into_iter()
            .map(|n| {
                n.into_iter()
                    .map(|n| (n.clone(), load_img_from_path(&n)))
                    .collect::<Vec<(String, Option<SharedPixelBuffer<Rgba8Pixel>>)>>()
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
    let client = Client::new();
    let mut handles = vec![];
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
                .clone()
                .into_iter()
                .zip(covers.into_iter())
                .map(|(n, c)| (n, c))
                .collect::<Vec<(String, String)>>();
            name_list.push(names);
            week_anime_list.push(anime_list);
        });
    Runtime::new().unwrap().block_on(async {
        for names in week_anime_list {
            for (n, c) in names {
                let handle = tokio::spawn(get_cover(c, n, client.clone()));
                handles.push(handle);
            }
        }
        for handle in handles {
            let _ = handle.await;
        }
    });
    serde_json::to_writer_pretty(std::fs::File::create(&anime_data_path).unwrap(), &name_list)
        .unwrap();
    let mut result = vec![];
    for names in &mut name_list {
        let mut temp = vec![];
        for n in names {
            temp.push((n.clone(), load_img_from_path(&n)));
        }
        result.push(temp);
    }
    result
}

async fn get_cover(cover: String, name: String, client: Client) {
    let save_path = APP_PATH.join("covers");
    let path = save_path.join(name).with_extension("jpg");
    if !path.exists() {
        let response = client.get(cover).send().await.unwrap();
        let bytes = response.bytes().await.unwrap();
        let mut file = std::fs::File::create(&path).unwrap();
        file.write_all(&bytes).unwrap();
    }
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

fn load_img_from_path(name: &str) -> Option<SharedPixelBuffer<Rgba8Pixel>> {
    let path = APP_PATH.join("covers").join(name).with_extension("jpg");
    let img = image::open(&path).unwrap().into_rgba8();
    let buffer =
        SharedPixelBuffer::<Rgba8Pixel>::clone_from_slice(img.as_raw(), img.width(), img.height());
    Some(buffer)
}

fn get_suffix(date: SlintDate) -> String {
    let suffix = match date.month {
        10 => format!("{}{}", date.year, date.month),
        _ => format!("{}0{}", date.year, date.month),
    };
    suffix.into()
}

pub fn init_anime_schedule(app: Weak<AppWindow>) -> SlintDate {
    let app = app.unwrap();
    let date = chrono::Local::now().date_naive();
    let mut anime_schedule = app.global::<AnimeData>().get_anime_schedule();
    anime_schedule.year = date.year();
    anime_schedule.month = match date.month() {
        1..=3 => 1,
        4..=6 => 4,
        7..=9 => 7,
        10..=12 => 10,
        _ => {
            panic!("Invalid month")
        } //TODO: 错误处理
    };
    app.global::<AnimeData>()
        .set_anime_schedule(anime_schedule.clone());
    anime_schedule
}
