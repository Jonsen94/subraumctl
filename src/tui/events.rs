//! Crossterm input thread.
//!
//! Reading terminal events blocks, so it lives on a dedicated thread and
//! forwards events into the async TUI loop.

use std::time::Duration;

use crossterm::event::{self, Event, KeyEvent, KeyEventKind};
use tokio::sync::mpsc::UnboundedSender;

#[derive(Debug)]
pub enum UiEvent {
    Key(KeyEvent),
    Resize(u16, u16),
}

pub fn spawn_input_thread(tx: UnboundedSender<UiEvent>) {
    std::thread::Builder::new()
        .name("subraumctl-input".to_string())
        .spawn(move || {
            loop {
                if tx.is_closed() {
                    break;
                }
                match event::poll(Duration::from_millis(50)) {
                    Ok(true) => match event::read() {
                        Ok(Event::Key(key))
                            if key.kind == KeyEventKind::Press
                                || key.kind == KeyEventKind::Repeat =>
                        {
                            let _ = tx.send(UiEvent::Key(key));
                        }
                        Ok(Event::Resize(width, height)) => {
                            let _ = tx.send(UiEvent::Resize(width, height));
                        }
                        Ok(_) => {}
                        Err(_) => break,
                    },
                    Ok(false) => {}
                    Err(_) => break,
                }
            }
        })
        .expect("failed to spawn TUI input thread");
}
