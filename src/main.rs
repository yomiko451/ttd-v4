slint::include_modules!();

mod logic;

fn main() -> Result<(), slint::PlatformError> {
    let app = AppWindow::new()?;
    let weak = app.as_weak();
    logic::init();
    logic::anime_logic(weak);
    app.run()
}
