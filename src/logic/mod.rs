mod anime;
mod todo;
mod init;
mod model;

pub use anime::{get_anime, init_anime_schedule, set_anime_logic};
pub use init::{APP_PATH, init};
pub use todo::{CURRENT_DATE, init_calendar};
