slint::include_modules!();

mod logic;

fn main() -> Result<(), slint::PlatformError> {
    let app = AppWindow::new()?;
    let weak = app.as_weak();
    logic::init(weak.clone());
    logic::set_anime_logic(weak.clone());
    logic::set_todo_logic(weak.clone());
    app.run()
}
