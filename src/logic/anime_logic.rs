use crate::logic::APP_PATH;
use scraper::{Html, Selector};
use slint::{invoke_from_event_loop, ComponentHandle, Image, Model, Weak};
use std::{io::Write, path::PathBuf, rc::Rc};
use crate::{Anime, AnimeData, AppWindow, DayAnime};

const BASE_URL: &str = "https://yuc.wiki/202501";

pub fn anime_logic(app: Weak<AppWindow>) {
    std::thread::spawn(||{
        invoke_from_event_loop(move || {
            let app = app.unwrap();
            let week_anime_list: Vec<DayAnime> = app.global::<AnimeData>().get_week_anime_list().iter().collect();
            let new_week_anime_list = parse_html(BASE_URL, week_anime_list);
            app.global::<AnimeData>().set_week_anime_list(Rc::new(slint::VecModel::from(new_week_anime_list)).into());
    }).unwrap();
    });
}


fn parse_html(url: &str, mut week_anime_list: Vec<DayAnime>) -> Vec<DayAnime> {
    let response = reqwest::blocking::get(url).unwrap();
    let html = response.text().unwrap();
    let document = Html::parse_document(&html);
    let selector = Selector::parse("div.post-body>div").unwrap();

    document.select(&selector).take(20).skip(1).step_by(3).enumerate().for_each(|(i, e)| {
        let name_selector = Selector::parse("tr:nth-child(1)>td").unwrap();
        let cover_selector = Selector::parse("div.div_date img").unwrap();
        let names = e.select(&name_selector).map(|ce| {
            ce.text().collect::<String>()
        }).collect::<Vec<String>>();
        let covers = e.select(&cover_selector).map(|ce| {
            ce.value().attr("data-src").unwrap_or("").to_string()
        }).collect::<Vec<String>>();
        let list = names.into_iter().zip(covers.into_iter()).map(|(n, c)|{
            let n = get_valid_filename(&n);
            let path = get_cover(&c, &n);
            let img = Image::load_from_path(&path).unwrap();
            Anime { name: n.into(), cover: img }
        }).collect::<Vec<Anime>>();
        let list = Rc::new(slint::VecModel::from(list));
        week_anime_list[i].anime_list = list.into();
    });
    week_anime_list
}

fn get_cover(cover: &str, name:&str) -> PathBuf {
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
fn get_valid_filename(name: &str) -> String{
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


#[cfg(test)]
mod tests {
    use super::*;

    
}