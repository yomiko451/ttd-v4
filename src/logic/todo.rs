use chrono::{NaiveDate, Datelike, Days, Local};
use slint::{ComponentHandle, Model, ModelRc, Weak};
use std::{rc::Rc, sync::LazyLock};
use crate::{logic::{SlintDate, APP_PATH}, AppWindow, Todo, TodoData, TodoKind };

pub const CURRENT_DATE: LazyLock<NaiveDate> = LazyLock::new(|| {
    Local::now().date_naive()
});

pub fn set_todo_logic(app: Weak<AppWindow>) {
    let app = app.unwrap();
    let todo_data = app.global::<TodoData>();
    let weak = app.as_weak();
    todo_data.on_update_calendar(move |new_date: SlintDate| {
        let app = weak.unwrap();
        let new_calendar = get_month_calendar(new_date.into());
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
    // 顺便初始化当前日期和当前日历
    todo_data.set_current_date((*CURRENT_DATE).into());
    todo_data.set_current_calendar((*CURRENT_DATE).into());
}

fn get_month_calendar(date: NaiveDate) -> Vec<Vec<SlintDate>> {
    let (year, month) = (date.year(), date.month());
    // 创建7个数组对应周一到周日
    let mut weekdays: Vec<Vec<SlintDate>> = vec![Vec::new(); 7];
    
    // 获取该月第一天
    let start_date = NaiveDate::from_ymd_opt(year, month, 1).unwrap();
    
    // 根据第一天星期几补0
    for i in 0..start_date.weekday().num_days_from_monday() {
        weekdays[i as usize].push(SlintDate::default());
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
        weekdays[weekday].push(SlintDate {
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
            weeks.push(SlintDate::default());
        }
    }

    weekdays
}

fn convert_vec_to_model(vec: Vec<Vec<SlintDate>>) -> ModelRc<ModelRc<SlintDate>> {
    let mut model = vec![];
    for i in vec {
        let m: ModelRc<SlintDate> = Rc::new(slint::VecModel::from(i)).into();
        model.push(m);
    };
    Rc::new(slint::VecModel::from(model)).into()
}

fn get_created_at_string() -> String {
    Local::now()
        .format("%Y-%m-%d %H:%M:%S")
        .to_string()
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


