use gtk::{glib, prelude::*};
use std::{cell::RefCell, path::Path, rc::Rc, sync::atomic::{AtomicUsize, Ordering}, time::{Duration, Instant}};

mod store {
    use super::*;
    pub static WRITES: AtomicUsize = AtomicUsize::new(0);
    #[derive(Clone)]
    pub struct Note { pub id: usize, pub text: String, pub modified: u64 }
    pub fn save(_: &Path, _: &[Note]) -> Result<(), std::io::Error> {
        WRITES.fetch_add(1, Ordering::SeqCst);
        std::thread::sleep(Duration::from_millis(500));
        Ok(())
    }
}
mod note {
    pub use super::store::Note;
}
#[path = "../../../../src/persistence/storage/save_queue.rs"]
mod save_queue;

fn main() {
    gtk::init().expect("GTK display required");
    let notes = Rc::new(RefCell::new((0..100).map(|id| store::Note { id, text: "x".repeat(1024), modified: 0 }).collect::<Vec<_>>()));
    let queue = Rc::new(RefCell::new(save_queue::SaveQueue::new("synthetic".into())));
    let buffer = gtk::TextBuffer::new(None);
    buffer.set_text(&"x".repeat(10 * 1024));
    let status = gtk::Label::new(None);
    let samples = Rc::new(RefCell::new(Vec::<Duration>::new()));
    let started = Instant::now();
    {
        let (notes, queue, samples) = (notes.clone(), queue.clone(), samples.clone());
        buffer.connect_changed(move |buffer| {
            let start = Instant::now();
            let text = buffer.text(&buffer.start_iter(), &buffer.end_iter(), true).to_string();
            if let Some(note) = notes.borrow_mut().iter_mut().find(|n| n.id == 99) {
                note.text = text;
                note.modified = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();
            }
            queue.borrow_mut().changed();
            status.set_text("저장 중…");
            samples.borrow_mut().push(start.elapsed());
        });
    }
    queue.borrow_mut().changed();
    queue.borrow_mut().dispatch(&notes.borrow());
    let main_loop = glib::MainLoop::new(None, false);
    let gaps = Rc::new(RefCell::new(Vec::new()));
    {
        let gaps = gaps.clone();
        let mut previous = Instant::now();
        glib::timeout_add_local(Duration::from_millis(5), move || {
            let now = Instant::now();
            gaps.borrow_mut().push(now.duration_since(previous));
            previous = now;
            glib::ControlFlow::Continue
        });
    }
    {
        let (queue, notes, main_loop, samples) = (queue.clone(), notes.clone(), main_loop.clone(), samples.clone());
        glib::timeout_add_local(Duration::from_millis(16), move || {
            let mut q = queue.borrow_mut();
            if let Some((_, result)) = q.poll() { result.expect("synthetic writer failed"); }
            if samples.borrow().len() == 100 && !q.in_flight() && !q.clean() { q.dispatch(&notes.borrow()); }
            if samples.borrow().len() == 100 && q.clean() { main_loop.quit(); return glib::ControlFlow::Break; }
            assert!(started.elapsed() < Duration::from_secs(5), "probe timed out");
            glib::ControlFlow::Continue
        });
    }
    {
        let mut count = 0;
        glib::timeout_add_local(Duration::from_millis(2), move || {
            buffer.insert_at_cursor("y");
            count += 1;
            if count == 100 { glib::ControlFlow::Break } else { glib::ControlFlow::Continue }
        });
    }
    main_loop.run();
    let mut samples = samples.borrow().clone();
    samples.sort();
    let p95 = samples[94];
    let max_gap = *gaps.borrow().iter().max().unwrap();
    let writes = store::WRITES.load(Ordering::SeqCst);
    println!("samples={} handler_p95_ms={:.3} heartbeat_max_gap_ms={:.3} writes={} writer_delay_ms=500 elapsed_ms={:.1}", samples.len(), p95.as_secs_f64()*1000.0, max_gap.as_secs_f64()*1000.0, writes, started.elapsed().as_secs_f64()*1000.0);
    assert_eq!(samples.len(), 100);
    assert_eq!(writes, 2, "one in-flight plus one coalesced write expected");
    assert!(p95 <= Duration::from_millis(8), "handler p95 exceeds target");
    assert!(max_gap <= Duration::from_millis(50), "heartbeat gap exceeds target");
}
