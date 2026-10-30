use super::state::State;
use gtk::Application;
pub(in crate::app) fn refresh_all(state: &State, app: &Application) {
    crate::app::library::refresh(state, app);
    crate::app::canvas::refresh(state);
}
