use crate::{
    AppWindow, CalendarDay, Filter, Todo, TodoData, TodoKind,
    logic::{APP_PATH, SlintDate},
    model::TODOS_MODEL,
};
use chrono::{Datelike, Days, Local, NaiveDate, Utc, Weekday};
use slint::{ComponentHandle, Model, ModelExt, ModelRc, SharedString, VecModel, Weak};
use std::{default, rc::Rc, sync::LazyLock};

pub const CURRENT_DATE: LazyLock<NaiveDate> = LazyLock::new(|| Local::now().date_naive());
pub const WEEKDAY: [Weekday; 7] = [
    Weekday::Mon,
    Weekday::Tue,
    Weekday::Wed,
    Weekday::Thu,
    Weekday::Fri,
    Weekday::Sat,
    Weekday::Sun,
];
pub fn set_todo_logic(app: Weak<AppWindow>) {
    let app = app.unwrap();
    let todo_data = app.global::<TodoData>();
    let weak = app.as_weak();
    todo_data.on_update_calendar(move |new_date: SlintDate| update_month(new_date, weak.clone()));
    let weak = app.as_weak();
    todo_data.on_add_todo(move |todo: Todo| add_todo(todo, weak.clone()));
    let weak = app.as_weak();
    todo_data.on_remove_todo(move |id: SharedString| remove_todo(id, weak.clone()));
    todo_data.on_filter_todos(filter_todo);
}

fn add_todo(mut todo: Todo, app: Weak<AppWindow>) {
    let app = app.unwrap();
    let todo_data = app.global::<TodoData>();
    todo.id = get_timestamp_id().into();
    todo.created_at = CURRENT_DATE.format("%Y-%m-%d").to_string().into();
    todo.days_to_start = calculate_days_to_start(&todo);
    TODOS_MODEL.with(|todos_model| todos_model.borrow_mut().add_todo(todo));
    todo_data
        .set_todo_list(TODOS_MODEL.with(|todos_model| todos_model.borrow().to_todo_list_model()));
    let new_calendar = TODOS_MODEL.with(|todos_model| todos_model.borrow().to_calendar_model());
    todo_data.set_calendar(new_calendar);
}

fn remove_todo(id: SharedString, app: Weak<AppWindow>) {
    let app = app.unwrap();
    let todo_data = app.global::<TodoData>();
    TODOS_MODEL.with(|todos_model| todos_model.borrow_mut().remove_todo(id));
    todo_data
        .set_todo_list(TODOS_MODEL.with(|todos_model| todos_model.borrow().to_todo_list_model()));
}

fn update_month(new_date: SlintDate, app: Weak<AppWindow>) {
    let app = app.unwrap();
    let todo_data = app.global::<TodoData>();
    let new_calendar = TODOS_MODEL.with(|todos_model| {
        todos_model.borrow_mut().update_month(new_date.into());
        todos_model.borrow().to_calendar_model()
    });
    todo_data.set_calendar(new_calendar);
}
pub fn init_todos(app: Weak<AppWindow>) {
    let app = app.unwrap();
    let todo_data = app.global::<TodoData>();
    // 初始化新待办
    let default_todo = todo_data.get_default_todo();
    todo_data.set_new_todo(default_todo);
    todo_data
        .set_calendar(TODOS_MODEL.with(|todos_model| todos_model.borrow().to_calendar_model()));
    todo_data
        .set_todo_list(TODOS_MODEL.with(|todos_model| todos_model.borrow().to_todo_list_model()));
    // 顺便初始化当前日期和当前选择日期
    todo_data.set_current_date((*CURRENT_DATE).into());
    todo_data.set_selected_date(
        TODOS_MODEL.with(|todos_model| todos_model.borrow().get_selected_date()),
    );
}

pub fn match_week_with_day(date: NaiveDate) -> Vec<Vec<u32>> {
    let (year, month) = (date.year(), date.month());
    // 创建7个数组对应周一到周日
    let mut weekdays = Vec::new();

    for _ in 0..7 {
        weekdays.push(Vec::new());
    }

    // 获取该月第一天
    let start_date = NaiveDate::from_ymd_opt(year, month, 1).unwrap();

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
        weekdays[weekday].push(current_date.day());

        // 前进一天
        current_date = current_date.checked_add_days(Days::new(1)).unwrap();
    }
    weekdays
}

fn calculate_days_to_start(todo: &Todo) -> i32 {
    match todo.kind {
        TodoKind::Once => {
            let date = NaiveDate::from(todo.once.clone());
            date.signed_duration_since(*CURRENT_DATE).num_days() as i32
        }
        TodoKind::Daily => 0,
        TodoKind::Weekly => {
            let weekday = todo.week as i32;
            let current_weekday = CURRENT_DATE.weekday() as i32;
            if current_weekday > weekday {
                7 - current_weekday + weekday
            } else {
                weekday - current_weekday
            }
        }
        TodoKind::Monthly => {
            let day = todo.day;
            let mut month = CURRENT_DATE.month();
            let day_now = CURRENT_DATE.day() as i32;
            if day_now <= day {
                day - day_now
            } else {
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
        }
        _ => 0,
    }
}

fn get_timestamp_id() -> String {
    Utc::now().timestamp().to_string()
}

fn filter_todo(filter: Filter, model: ModelRc<Todo>) -> ModelRc<Todo> {
    match filter {
        Filter::All => model,
        Filter::Once => {
            let vec = model
                .iter()
                .filter(|t| t.kind == TodoKind::Once)
                .collect::<Vec<Todo>>();
            Rc::new(slint::VecModel::from(vec)).into()
        }
        Filter::Daily => {
            let vec = model
                .iter()
                .filter(|t| t.kind == TodoKind::Daily)
                .collect::<Vec<Todo>>();
            Rc::new(slint::VecModel::from(vec)).into()
        }
        Filter::Weekly => {
            let vec = model
                .iter()
                .filter(|t| t.kind == TodoKind::Weekly)
                .collect::<Vec<Todo>>();
            Rc::new(slint::VecModel::from(vec)).into()
        }
        Filter::Monthly => {
            let vec = model
                .iter()
                .filter(|t| t.kind == TodoKind::Monthly)
                .collect::<Vec<Todo>>();
            Rc::new(slint::VecModel::from(vec)).into()
        }
        Filter::Progress => {
            let vec = model
                .iter()
                .filter(|t| t.kind == TodoKind::Progress)
                .collect::<Vec<Todo>>();
            Rc::new(slint::VecModel::from(vec)).into()
        }
        _ => model,
    }
}
