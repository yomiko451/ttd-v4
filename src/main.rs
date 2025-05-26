slint::include_modules!();

mod logic;
mod model;

fn main() -> Result<(), slint::PlatformError> {
    let app = AppWindow::new()?;
    let weak = app.as_weak();
    logic::set_anime_logic(weak.clone());
    logic::set_todo_logic(weak.clone());
    logic::init(weak.clone());
    app.window().on_close_requested(move || {
        slint::CloseRequestResponse::HideWindow // TODO: 完善关闭逻辑
    });
    app.run()
}
