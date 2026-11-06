mod app;
#[cfg(test)]
mod architecture_tests;
mod persistence;
mod presentation;

fn main() {
    app::run();
}
