mod canvas;
mod editor;
mod files;
#[cfg(test)]
mod gtk_tests;
mod library;
mod shared;
mod shell;
#[cfg(test)]
mod test_support;
#[cfg(test)]
mod visual_reference;
mod widgets;

use gtk::{prelude::*, Application};

pub(crate) fn run() {
    // 전환 기간에는 GTK와 SQLite 앱이 같은 원본을 동시에 변경하지 못하게 한다.
    let _data_lock = match legacy_data_lock() {
        Ok(lock) => lock,
        Err(error) => {
            eprintln!("MemoIvy: {error}");
            return;
        }
    };
    let app = Application::builder()
        .application_id("io.github.pollux_o4.MemoIvy")
        .build();
    app.connect_startup(|_| crate::presentation::install());
    app.connect_activate(|app| {
        if let Some(w) = app
            .windows()
            .into_iter()
            .find(|w| w.title().as_deref() == Some("MemoIvy"))
        {
            w.present();
        } else {
            library::build(app);
        }
    });
    app.run();
}

fn legacy_data_lock() -> Result<std::fs::File, String> {
    let notes = crate::persistence::data_path()?;
    let directory = notes.parent().ok_or("저장 폴더를 찾을 수 없습니다")?;
    std::fs::create_dir_all(directory).map_err(|error| error.to_string())?;
    let lock = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(directory.join(".memoivy.lock"))
        .map_err(|error| error.to_string())?;
    lock.try_lock()
        .map_err(|_| "다른 MemoIvy가 같은 데이터를 사용하고 있습니다".to_string())?;
    if directory.join("memoivy.sqlite3").exists() {
        return Err("이 데이터는 새 데스크톱 앱으로 전환되었습니다. 새 앱을 실행하세요".into());
    }
    Ok(lock)
}
