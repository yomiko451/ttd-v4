use chrono::{Datelike, NaiveDate, Weekday};
use slint::{Model, ModelRc, SharedString, VecModel};
use std::{
    cell::{LazyCell, RefCell},
    collections::HashMap,
    rc::Rc,
    sync::LazyLock,
};

use crate::{
    CalendarDay, Todo, TodoKind,
    logic::{APP_PATH, CURRENT_DATE, SlintDate, WEEKDAY, match_week_with_day},
};

thread_local! {
    pub static TODOS_MODEL: Rc<RefCell<TodosModel>> = {
    let path = APP_PATH.join("data").join("todo_list.json");
    let todos_model = if path.exists() {
        let todos: Vec<Todo> = serde_json::from_reader(std::fs::File::open(path).unwrap()).unwrap();
        let mut new_todo_model = TodosModel::new(*CURRENT_DATE);
        new_todo_model.load_todos(todos);
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

    fn load_todos(&mut self, todos: Vec<Todo>) {
        for todo in todos {
            self.add_todo(todo);
        }
    }

    pub fn get_selected_date(&self) -> SlintDate {
        self.selected_date.into()
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
                date: date.into(),
                todo_list: model,
            });
        }
        while calendar.len() < 42 {
            calendar.push(CalendarDay::default());
        }
        Rc::new(VecModel::from(calendar)).into()
    }

    pub fn remove_todo(&mut self, id: SharedString) {
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

    pub fn add_todo(&mut self, todo: Todo) {
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
                let once: NaiveDate = todo.borrow().once.clone().into();
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
                let start = todo.borrow().start_date.clone().into();
                let end = todo.borrow().end_date.clone().into();
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
                let start = todo.borrow().start_date.clone().into();
                let end = todo.borrow().end_date.clone().into();
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
                let start = todo.borrow().start_date.clone().into();
                let end = todo.borrow().end_date.clone().into();
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

    pub fn update_month(&mut self, date: NaiveDate) {
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
