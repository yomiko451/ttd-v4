use crate::{
    AppWindow, Filter, Todo, TodoData, TodoKind,
    logic::{APP_PATH, SlintDate},CalendarDay
};
use chrono::{Datelike, Days, Local, NaiveDate, Utc, Weekday};
use slint::{ComponentHandle, Model, ModelRc, SharedString, VecModel, Weak};
use std::{rc::Rc, sync::LazyLock, collections::HashMap, cell::RefCell};

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

thread_local! {
    pub static TODOS_MODEL: Rc<RefCell<TodosModel>> = {
    let path = APP_PATH.join("data").join("todo_list.json");
    let todos_model = if path.exists() {
        let todos: Vec<Todo> = serde_json::from_reader(std::fs::File::open(path).unwrap()).unwrap();
        let mut new_todo_model = TodosModel::new(*CURRENT_DATE);
        new_todo_model.load_todos_to_model(todos);
        new_todo_model
    } else {
        TodosModel::new(*CURRENT_DATE)
    };
    Rc::new(RefCell::new(todos_model))
};
}

#[derive(Debug, Default)]
pub struct TodosModel {
    // 日历显示的月份，默认是当前日期所在月份
    selected_date: NaiveDate,
    // 每个待办id要在一个月内哪些日期中显示
    id_date_map: HashMap<String, Vec<NaiveDate>>,
    // 每个id对应哪条待办
    id_todo_map: HashMap<String, Rc<RefCell<Todo>>>,
    // 一个月内每个日期包括哪些待办
    date_todo_map: HashMap<NaiveDate, Vec<Rc<RefCell<Todo>>>>,
    // 一个月内星期一至星期天对应哪些日期
    week_day_map: HashMap<Weekday, Vec<u32>>,
}

impl TodosModel {
    fn new(selected_date: NaiveDate) -> Self {
        let id_map = HashMap::new();
        let mut date_calendar_map = HashMap::new();
        for d in 1..=selected_date.num_days_in_month() {
            let date =
                NaiveDate::from_ymd_opt(selected_date.year(), selected_date.month(), d as u32);
            date_calendar_map.insert(date.unwrap(), vec![]);
        }
        let mut week_calendar_map = HashMap::new();
        let days = match_week_with_day(selected_date);
        for (w, d) in WEEKDAY.iter().zip(days) {
            week_calendar_map.insert(*w, d);
        }
        let id_dates = HashMap::new();
        TodosModel {
            id_todo_map: id_map,
            date_todo_map: date_calendar_map,
            week_day_map: week_calendar_map,
            selected_date,
            id_date_map: id_dates,
        }
    }

    fn load_todos_to_model(&mut self, todos: Vec<Todo>) {
        for mut todo in todos {
            //加载时顺便检测是否过期
            if todo.calculate_days_to_start().is_none() {
                todo.is_expired = true;
            } 
            self.add_todo_model(todo);
        }
    }

    pub fn get_selected_date(&self) -> SlintDate {
        SlintDate::from_naive_date(&self.selected_date)
    }
    pub fn to_todo_list_model(&self) -> ModelRc<Todo> {
        let mut todos_vec = self
            .id_todo_map
            .values()
            .map(|t| t.borrow().clone())
            .collect::<Vec<Todo>>();
        todos_vec.sort_by_key(|t| t.id.parse::<i64>().unwrap());
        let modelrc = Rc::new(VecModel::from(todos_vec));
        modelrc.into()
    }

    pub fn to_calendar_model(&self) -> ModelRc<CalendarDay> {
        let mut calendar = vec![];
        let start =
            NaiveDate::from_ymd_opt(self.selected_date.year(), self.selected_date.month(), 1)
                .unwrap();
        let week_of_start = start.weekday().num_days_from_monday();
        let days_of_month = self.selected_date.num_days_in_month();
        for _ in 0..week_of_start {
            calendar.push(CalendarDay::default());
        }
        for i in 1..=days_of_month {
            let date = NaiveDate::from_ymd_opt(
                self.selected_date.year(),
                self.selected_date.month(),
                i as u32,
            )
            .unwrap();
            let todo_list = self
                .date_todo_map
                .get(&date)
                .unwrap()
                .iter()
                .map(|t| t.borrow().to_owned())
                .collect::<Vec<Todo>>();
            let model = Rc::new(VecModel::from(todo_list)).into();
            calendar.push(CalendarDay {
                date: SlintDate::from_naive_date(&date),
                todo_list: model,
            });
        }
        while calendar.len() < 42 {
            calendar.push(CalendarDay::default());
        }
        Rc::new(VecModel::from(calendar)).into()
    }

    pub fn remove_todo_from_model(&mut self, id: SharedString) {
        if let Some(todo) = self.id_todo_map.remove(&id.to_string()) {
            for date in self
                .id_date_map
                .get(&todo.borrow().id.to_string())
                .unwrap()
                .iter()
            {
                self.date_todo_map
                    .get_mut(&date)
                    .unwrap()
                    .retain(|t| t.borrow().id != todo.borrow().id);
            }
            self.id_date_map.remove(&todo.borrow().id.to_string());
            self.save_todos();
        }
    }

    pub fn add_todo_model(&mut self, todo: Todo) {
        let todo = Rc::new(RefCell::new(todo));
        self.id_todo_map
            .insert(todo.borrow().id.to_string(), todo.clone());
        self.id_date_map
            .insert(todo.borrow().id.to_string(), Vec::new());
        self.match_todo_with_calendar(todo);
        self.save_todos();
    }

    fn match_todo_with_calendar(&mut self, todo: Rc<RefCell<Todo>>) {
        match todo.borrow().kind {
            TodoKind::Once => {
                let once: NaiveDate = todo.borrow().once.to_naive_date();
                if once.year() == self.selected_date.year()
                    && once.month() == self.selected_date.month()
                {
                    self.id_date_map
                        .get_mut(&todo.borrow().id.to_string())
                        .unwrap()
                        .push(once);
                    self.date_todo_map
                        .get_mut(&once)
                        .unwrap()
                        .push(todo.clone());
                }
            }
            TodoKind::Daily | TodoKind::Progress => {
                let start = todo.borrow().start_date.to_naive_date();
                let end = todo.borrow().end_date.to_naive_date();
                let keys = self
                    .date_todo_map
                    .keys()
                    .copied()
                    .collect::<Vec<NaiveDate>>();
                for key in keys {
                    if key < start || key > end {
                        continue;
                    }
                    self.id_date_map
                        .get_mut(&todo.borrow().id.to_string())
                        .unwrap()
                        .push(key);
                    self.date_todo_map.get_mut(&key).unwrap().push(todo.clone())
                }
            }
            TodoKind::Weekly => {
                let start = todo.borrow().start_date.to_naive_date();
                let end = todo.borrow().end_date.to_naive_date();
                let week = todo.borrow().week.into();
                if let Some(days) = self.week_day_map.get(&week) {
                    for day in days {
                        let date = NaiveDate::from_ymd_opt(
                            self.selected_date.year(),
                            self.selected_date.month(),
                            *day,
                        )
                        .unwrap();
                        if date < start || date > end {
                            continue;
                        }
                        self.id_date_map
                            .get_mut(&todo.borrow().id.to_string())
                            .unwrap()
                            .push(date);
                        self.date_todo_map
                            .get_mut(&date)
                            .unwrap()
                            .push(todo.clone());
                    }
                }
            }
            TodoKind::Monthly => {
                let start = todo.borrow().start_date.to_naive_date();
                let end = todo.borrow().end_date.to_naive_date();
                let date = NaiveDate::from_ymd_opt(
                    self.selected_date.year(),
                    self.selected_date.month(),
                    todo.borrow().day as u32,
                )
                .unwrap();
                if date < start || date > end {
                    return;
                }
                self.id_date_map
                    .get_mut(&todo.borrow().id.to_string())
                    .unwrap()
                    .push(date);
                self.date_todo_map
                    .get_mut(&date)
                    .unwrap()
                    .push(todo.clone());
            }
        }
    }

    pub fn update_calendar_for_model(&mut self, date: NaiveDate) {
        self.selected_date = date;
        self.date_todo_map.clear();
        self.week_day_map.clear();
        self.id_date_map.clear();
        for d in 1..=self.selected_date.num_days_in_month() {
            let date = NaiveDate::from_ymd_opt(
                self.selected_date.year(),
                self.selected_date.month(),
                d as u32,
            );
            self.date_todo_map.insert(date.unwrap(), vec![]);
        }
        let days = match_week_with_day(self.selected_date);
        for (w, d) in WEEKDAY.iter().zip(days) {
            self.week_day_map.insert(*w, d);
        }
        let todos = self
            .id_todo_map
            .values()
            .map(|t| t.clone())
            .collect::<Vec<_>>();
        for todo in todos {
            self.id_date_map
                .insert(todo.borrow().id.to_string(), Vec::new());
            self.match_todo_with_calendar(todo);
        }
    }

    fn save_todos(&self) {
        let todos = self
            .id_todo_map
            .values()
            .map(|t| t.borrow().to_owned())
            .collect::<Vec<Todo>>();
        let path = APP_PATH.join("data").join("todo_list.json");
        let file = std::fs::File::create(path).unwrap();
        serde_json::to_writer(file, &todos).unwrap();
    }
}

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
    todo_data.on_duration_check(|mut todo| todo.calculate_days_to_start().is_some());
}

fn add_todo(mut todo: Todo, app: Weak<AppWindow>) {
    let app = app.unwrap();
    let todo_data = app.global::<TodoData>();
    todo.set_timestamp_id();
    todo.created_at = CURRENT_DATE.format("%Y-%m-%d").to_string().into();
    todo.calculate_days_to_start(); // TODO none返回错误
    TODOS_MODEL.with(|todos_model| todos_model.borrow_mut().add_todo_model(todo));
    todo_data
        .set_todo_list(TODOS_MODEL.with(|todos_model| todos_model.borrow().to_todo_list_model()));
    let new_calendar = TODOS_MODEL.with(|todos_model| todos_model.borrow().to_calendar_model());
    todo_data.set_calendar(new_calendar);
}

fn remove_todo(id: SharedString, app: Weak<AppWindow>) {
    let app = app.unwrap();
    let todo_data = app.global::<TodoData>();
    TODOS_MODEL.with(|todos_model| todos_model.borrow_mut().remove_todo_from_model(id));
    todo_data
        .set_todo_list(TODOS_MODEL.with(|todos_model| todos_model.borrow().to_todo_list_model()));
    let new_calendar = TODOS_MODEL.with(|todos_model| todos_model.borrow().to_calendar_model());
    todo_data.set_calendar(new_calendar);
}

fn update_month(new_date: SlintDate, app: Weak<AppWindow>) {
    let app = app.unwrap();
    let todo_data = app.global::<TodoData>();
    let new_calendar = TODOS_MODEL.with(|todos_model| {
        todos_model.borrow_mut().update_calendar_for_model(new_date.to_naive_date());
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
    todo_data.set_current_date(SlintDate::from_naive_date(&CURRENT_DATE));
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

impl Todo {
    fn set_timestamp_id(&mut self) {
    self.id = Utc::now().timestamp().to_string().into();
}

    fn calculate_days_to_start(&mut self) -> Option<i32> {
    let days = match self.kind {
        TodoKind::Once => {
            let date = self.once.to_naive_date();
            let days = date.signed_duration_since(*CURRENT_DATE).num_days() as i32;
            self.days_to_start = days;
            if days > 0 {
                return Some(days);
            } else {
                return None;
            }
        }
        TodoKind::Daily | TodoKind::Progress => 0,
        TodoKind::Weekly => {
            let weekday = self.week as i32;
            let current_weekday = CURRENT_DATE.weekday() as i32;
            if current_weekday > weekday {
                7 - current_weekday + weekday
            } else {
                weekday - current_weekday
            }
        }
        TodoKind::Monthly => {
            let day = self.day;
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
    };
    self.days_to_start = days;
    let next_date = CURRENT_DATE.checked_add_days(Days::new(days as u64)).unwrap();
    if next_date > self.end_date.to_naive_date() {
        None
    } else {
        Some(days)
    }
}
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
        Filter::Today => {
            let today = *CURRENT_DATE;
            let vec = model
                .iter()
                .filter(|t| {
                    match t.kind {
                        TodoKind::Once => t.once.to_naive_date() == today,
                        TodoKind::Daily | TodoKind::Progress => t.start_date.to_naive_date() <= today && t.end_date.to_naive_date() >= today,
                        TodoKind::Weekly => {
                            let week = t.week as u32;
                            let current_weekday = today.weekday() as u32;
                            t.start_date.to_naive_date() <= today && t.end_date.to_naive_date() >= today && week == current_weekday
                        }
                        TodoKind::Monthly => t.start_date.to_naive_date() <= today && t.end_date.to_naive_date() >= today && t.day == today.day() as i32,
                    }
                })
                .collect::<Vec<Todo>>();
            Rc::new(slint::VecModel::from(vec)).into()
        },
    }
}
