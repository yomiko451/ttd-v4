use chrono::{Datelike, Weekday};
use slint::ModelRc;

mod anime;
mod init;
mod todo;

pub use crate::Date as SlintDate;
use crate::SlintWeekday;
pub use anime::{get_anime, init_anime_schedule, set_anime_logic};
pub use init::{APP_PATH, init};
use serde::{Deserialize, Serialize};
pub use todo::{CURRENT_DATE, WEEKDAY, init_todos, match_week_with_day, set_todo_logic};

impl Serialize for SlintDate {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let date_str = format!("{}-{}-{}", self.year, self.month, self.day);
        serializer.serialize_str(&date_str)
    }
}

impl<'de> Deserialize<'de> for SlintDate {
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
        Ok(SlintDate { year, month, day })
    }
}

impl PartialOrd for SlintDate {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        if self.year < other.year {
            return Some(std::cmp::Ordering::Less);
        } else if self.year > other.year {
            return Some(std::cmp::Ordering::Greater);
        } else if self.month < other.month {
            return Some(std::cmp::Ordering::Less);
        } else if self.month > other.month {
            return Some(std::cmp::Ordering::Greater);
        } else if self.day < other.day {
            return Some(std::cmp::Ordering::Less);
        } else if self.day > other.day {
            return Some(std::cmp::Ordering::Greater);
        } else {
            return Some(std::cmp::Ordering::Equal);
        }
    }
}

impl From<chrono::NaiveDate> for SlintDate {
    fn from(date: chrono::NaiveDate) -> Self {
        SlintDate {
            year: date.year(),
            month: date.month() as i32,
            day: date.day() as i32,
        }
    }
}

impl From<SlintDate> for chrono::NaiveDate {
    fn from(date: SlintDate) -> Self {
        chrono::NaiveDate::from_ymd_opt(date.year, date.month as u32, date.day as u32).unwrap()
    }
}

impl From<Weekday> for SlintWeekday {
    fn from(weekday: Weekday) -> Self {
        match weekday {
            Weekday::Mon => SlintWeekday::Monday,
            Weekday::Tue => SlintWeekday::Tuesday,
            Weekday::Wed => SlintWeekday::Wednesday,
            Weekday::Thu => SlintWeekday::Thursday,
            Weekday::Fri => SlintWeekday::Friday,
            Weekday::Sat => SlintWeekday::Saturday,
            Weekday::Sun => SlintWeekday::Sunday,
        }
    }
}

impl From<SlintWeekday> for Weekday {
    fn from(weekday: SlintWeekday) -> Self {
        match weekday {
            SlintWeekday::Monday => Weekday::Mon,
            SlintWeekday::Tuesday => Weekday::Tue,
            SlintWeekday::Wednesday => Weekday::Wed,
            SlintWeekday::Thursday => Weekday::Thu,
            SlintWeekday::Friday => Weekday::Fri,
            SlintWeekday::Saturday => Weekday::Sat,
            SlintWeekday::Sunday => Weekday::Sun,
        }
    }
}
