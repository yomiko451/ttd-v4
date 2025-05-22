mod anime;
mod todo;
mod init;

pub use anime::{get_anime, init_anime_schedule, set_anime_logic};
pub use init::{APP_PATH, init};
pub use todo::{CURRENT_DATE, init_calendar, set_todo_logic};
