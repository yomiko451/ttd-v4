use chrono::{NaiveDate, Datelike, Days, Local};
use slint::{ComponentHandle, ModelRc, Weak};
use std::{rc::Rc, sync::LazyLock};
use crate::{AppWindow, Date, TodoData };

pub const CURRENT_DATE: LazyLock<NaiveDate> = LazyLock::new(|| {
    Local::now().date_naive()
});

pub fn set_todo_logic(app: Weak<AppWindow>) {
    let app = app.unwrap();
    let todo_data = app.global::<TodoData>();
    let weak = app.as_weak();
    todo_data.on_update_calendar(move || {
        let app = weak.unwrap();
        let todo_data = app.global::<TodoData>();
        let new_date = todo_data.get_current_date();
        let new_calendar = get_month_calendar(convert_date_to_naivedate(new_date));
        let model = convert_vec_to_model(new_calendar);
        todo_data.set_calendar(model);
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

fn get_month_calendar(date: NaiveDate) -> Vec<Vec<i32>> {
    let (year, month) = (date.year(), date.month());
    // 创建7个数组对应周一到周日
    let mut weekdays: Vec<Vec<i32>> = vec![Vec::new(); 7];
    
    // 获取该月第一天
    let start_date = NaiveDate::from_ymd_opt(year, month, 1).unwrap();
    
    // 根据第一天星期几补0
    for i in 0..start_date.weekday().num_days_from_monday() {
        weekdays[i as usize].push(0);
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
        weekdays[weekday].push(current_date.day() as i32);
        
        // 前进一天
        current_date = current_date.checked_add_days(Days::new(1)).unwrap();
    }

    // 长度不足6的补0
    for weeks in weekdays.iter_mut() {
        while weeks.len() < 6 {
            weeks.push(0);
        }
    }

    weekdays
}

fn convert_vec_to_model(vec: Vec<Vec<i32>>) -> ModelRc<ModelRc<i32>> {
    let mut model = vec![];
    for i in vec {
        let m: ModelRc<i32> = Rc::new(slint::VecModel::from(i)).into();
        model.push(m);
    };
    Rc::new(slint::VecModel::from(model)).into()
}

fn convert_date_to_naivedate(date: Date) -> NaiveDate {
    NaiveDate::from_ymd_opt(date.year, date.month as u32, date.day as u32).unwrap()
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