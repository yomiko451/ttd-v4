use chrono::{NaiveDate, Datelike, Days, Local};
use image::imageops::FilterType::CatmullRom;
use serde::{Deserialize, Serialize};
use slint::{ComponentHandle, Model, ModelRc, Weak};
use std::{rc::Rc, sync::LazyLock};
use crate::{logic::APP_PATH, AppWindow, Date, Todo, TodoData };

pub const CURRENT_DATE: LazyLock<NaiveDate> = LazyLock::new(|| {
    Local::now().date_naive()
});

pub fn set_todo_logic(app: Weak<AppWindow>) {
    let app = app.unwrap();
    let todo_data = app.global::<TodoData>();
    let weak = app.as_weak();
    todo_data.on_update_calendar(move |new_date: Date| {
        let app = weak.unwrap();
        let new_calendar = get_month_calendar(convert_date_to_naivedate(new_date));
        let model = convert_vec_to_model(new_calendar);
        app.global::<TodoData>().set_calendar(model);
    });
    let weak = app.as_weak();
    todo_data.on_add_todo(move |mut todo: Todo| {
        let app = weak.unwrap();
        let todo_data = app.global::<TodoData>();
        let mut todos = todo_data.get_todo_list().iter().collect::<Vec<Todo>>();
        todo.created_at = get_created_at_string().into();
        todo.days_to_start = calculate_days_to_start(&todo);
        todos.push(todo);
        save_todos(&todos);
        todo_data.set_todo_list(Rc::new(slint::VecModel::from(todos)).into());
    });
}

pub fn init_calendar(app: Weak<AppWindow>) {
    let app = app.unwrap();
    let todo_data = app.global::<TodoData>();
    let current_date = get_month_calendar(*CURRENT_DATE);
    let model = convert_vec_to_model(current_date);
    todo_data.set_calendar(model);
    // 顺便初始化当前日期
    todo_data.set_current_date(get_current_date());
}

fn get_month_calendar(date: NaiveDate) -> Vec<Vec<Date>> {
    let (year, month) = (date.year(), date.month());
    // 创建7个数组对应周一到周日
    let mut weekdays: Vec<Vec<Date>> = vec![Vec::new(); 7];
    
    // 获取该月第一天
    let start_date = NaiveDate::from_ymd_opt(year, month, 1).unwrap();
    
    // 根据第一天星期几补0
    for i in 0..start_date.weekday().num_days_from_monday() {
        weekdays[i as usize].push(Date::default());
    }

    // 获取下个月第一天
    let next_month = if month == 12 {
        NaiveDate::from_ymd_opt(year + 1, 1, 1).unwrap()
    } else {
        NaiveDate::from_ymd_opt(year, month + 1, 1).unwrap()
    };

    // 遍历该月的每一天
    let mut current_date = start_date;
    while current_date < next_month {
        // weekday()返回0-6，其中0是周一
        let weekday = current_date.weekday().num_days_from_monday() as usize;
        weekdays[weekday].push(Date {
            year,
            month: month as i32,
            day: current_date.day() as i32,
        });
        
        // 前进一天
        current_date = current_date.checked_add_days(Days::new(1)).unwrap();
    }

    // 长度不足6的补0
    for weeks in weekdays.iter_mut() {
        while weeks.len() < 6 {
            weeks.push(Date::default());
        }
    }

    weekdays
}

fn convert_vec_to_model(vec: Vec<Vec<Date>>) -> ModelRc<ModelRc<Date>> {
    let mut model = vec![];
    for i in vec {
        let m: ModelRc<Date> = Rc::new(slint::VecModel::from(i)).into();
        model.push(m);
    };
    Rc::new(slint::VecModel::from(model)).into()
}

fn convert_date_to_naivedate(date: Date) -> NaiveDate {
    NaiveDate::from_ymd_opt(date.year, date.month as u32, date.day as u32).unwrap()
}

fn get_created_at_string() -> String {
    Local::now()
        .format("%Y-%m-%d %H:%M:%S")
        .to_string()
}

fn calculate_days_to_start(todo: &Todo) -> i32 {
    match todo.kind {
        0 => {
            let date = convert_date_to_naivedate(todo.once.clone());
            date.signed_duration_since(*CURRENT_DATE).num_days() as i32
        }
        1 => 0,
        2 => {
            let weekday = todo.week as i32;
            let current_weekday = CURRENT_DATE.weekday() as i32;
            if current_weekday > weekday {
                7 - current_weekday + weekday
            } else {
                weekday - current_weekday
            }
        }
        3 => {
            let day = todo.day;
                    let mut month = CURRENT_DATE.month();
                    let day_now = CURRENT_DATE.day() as i32;
                    if day_now <= day {
                        day - day_now
                    } else {
                        println!("{}", day);
                        let next_date = loop {
                            let (y, m) = match month {
                                12 => (CURRENT_DATE.year() + 1, 1),
                                m => (CURRENT_DATE.year(), m + 1),
                            };
                            let date = NaiveDate::from_ymd_opt(y, m, day as u32);
                            if date.is_some() {
                                break date.unwrap();
                            } else {
                                month += 1;
                            }
                        };
                        let days = next_date.signed_duration_since(*CURRENT_DATE).num_days();
                        days as i32
                    }
            },
        _ => 0,
    }
}

fn save_todos(todos: &Vec<Todo>) {
    let path = APP_PATH.join("data").join("todo_lsit.json");
    let file = std::fs::File::create(path).unwrap();
    serde_json::to_writer(file, &todos).unwrap();
}

pub fn load_todos(app: Weak<AppWindow>) {
    let app = app.unwrap();
    let path = APP_PATH.join("data").join("todo_lsit.json");
    if path.exists() {
        let todos: Vec<Todo> = serde_json::from_reader(std::fs::File::open(path).unwrap()).unwrap();
        let model = Rc::new(slint::VecModel::from(todos));
        app.global::<TodoData>().set_todo_list(model.into());
    }
}

impl Serialize for Date {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let date_str = format!("{}-{}-{}", self.year, self.month, self.day);
        serializer.serialize_str(&date_str)
    }
}

impl<'de> Deserialize<'de> for Date {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let date_str = String::deserialize(deserializer)?;
        let parts: Vec<&str> = date_str.split('-').collect();
        if parts.len() != 3 {
            return Err(serde::de::Error::custom("Invalid date format"));
        }
        let year = parts[0].parse::<i32>().map_err(serde::de::Error::custom)?;
        let month = parts[1].parse::<i32>().map_err(serde::de::Error::custom)?;
        let day = parts[2].parse::<i32>().map_err(serde::de::Error::custom)?;
        Ok(Date { year, month, day })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_get_month_calendar() {
    let date = chrono::Local::now().date_naive();
    let calendar = get_month_calendar(date);
    let days = ["周一", "周二", "周三", "周四", "周五", "周六", "周日"];
    for (i, week) in calendar.iter().enumerate() {
        println!("{}: {:?}", days[i], week);
    }
}
}

fn get_current_date() -> Date {
    let date = *CURRENT_DATE;
    Date {
        year: date.year(),
        month: date.month() as i32,
        day: date.day() as i32,
    }
}