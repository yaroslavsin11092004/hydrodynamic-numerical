mod raii_objects;
mod engine;
use engine::ripple_view;
fn main() {
    let mut window = ripple_view::RippleView::new(900, 700, 14).expect("Failed to create RippleView");
    window.main_loop();
}
