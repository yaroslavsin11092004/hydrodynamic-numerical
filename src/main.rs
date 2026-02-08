mod raii_objects;
mod engine;
mod ui;
use engine::ripple_view;
fn main() {
    let mut window = ripple_view::RippleView::new(900, 700, 18).expect("Failed to create RippleView");
    window.main_loop();
}
