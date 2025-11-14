//! Event handling and event loop for TUI

use crossterm::event::{self, KeyCode, KeyEvent, KeyModifiers, Event as CrosstermEvent};
use std::sync::mpsc::{channel, Sender, Receiver};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

/// UI events
#[derive(Debug, Clone)]
pub enum Event {
    /// Keyboard key pressed
    Key(KeyEvent),

    /// Terminal resized
    Resize(u16, u16),

    /// Tick (60fps update)
    Tick,

    /// Quit event
    Quit,

    /// Error event
    Error(String),
}

/// Event handler for keyboard and terminal events
pub struct EventHandler {
    /// Sender channel
    #[allow(dead_code)]
    sender: Sender<Event>,

    /// Receiver channel
    receiver: Receiver<Event>,

    /// Handle to event loop thread
    _thread_handle: Option<thread::JoinHandle<()>>,
}

impl EventHandler {
    /// Create a new event handler
    pub fn new() -> Self {
        let (sender, receiver) = channel();
        let sender_clone = sender.clone();

        let thread_handle = thread::spawn(move || {
            let tick_rate = Duration::from_millis(1000 / 60); // 60 FPS
            let mut last_tick = std::time::Instant::now();

            loop {
                let timeout = tick_rate
                    .checked_sub(last_tick.elapsed())
                    .unwrap_or_else(|| Duration::from_secs(0));

                if event::poll(timeout).unwrap_or(false) {
                    if let Ok(CrosstermEvent::Key(key)) = event::read() {
                        if sender_clone.send(Event::Key(key)).is_err() {
                            break;
                        }
                    } else if let Ok(CrosstermEvent::Resize(w, h)) = event::read() {
                        if sender_clone.send(Event::Resize(w, h)).is_err() {
                            break;
                        }
                    }
                }

                if last_tick.elapsed() >= tick_rate {
                    if sender_clone.send(Event::Tick).is_err() {
                        break;
                    }
                    last_tick = std::time::Instant::now();
                }
            }
        });

        Self {
            sender,
            receiver,
            _thread_handle: Some(thread_handle),
        }
    }

    /// Receive next event (blocking)
    pub fn next(&self) -> Result<Event, std::sync::mpsc::RecvError> {
        self.receiver.recv()
    }

    /// Receive next event (non-blocking)
    pub fn try_next(&self) -> Result<Event, std::sync::mpsc::TryRecvError> {
        self.receiver.try_recv()
    }
}

impl Default for EventHandler {
    fn default() -> Self {
        Self::new()
    }
}

/// Check if a key is a quit key (Ctrl+C, Ctrl+D, or 'q')
pub fn is_quit_key(key: KeyEvent) -> bool {
    matches!(
        (key.code, key.modifiers),
        (KeyCode::Char('c'), KeyModifiers::CONTROL)
            | (KeyCode::Char('d'), KeyModifiers::CONTROL)
            | (KeyCode::Char('q'), KeyModifiers::NONE)
            | (KeyCode::Esc, KeyModifiers::NONE)
    )
}

/// Check if a key is a refresh key (Ctrl+R)
pub fn is_refresh_key(key: KeyEvent) -> bool {
    matches!(
        (key.code, key.modifiers),
        (KeyCode::Char('r'), KeyModifiers::CONTROL)
    )
}

/// Check if a key is a help key (?)
pub fn is_help_key(key: KeyEvent) -> bool {
    matches!(key.code, KeyCode::Char('?') | KeyCode::Char('h') | KeyCode::F(1))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_quit_key_ctrl_c() {
        let key = KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL);
        assert!(is_quit_key(key));
    }

    #[test]
    fn test_is_quit_key_ctrl_d() {
        let key = KeyEvent::new(KeyCode::Char('d'), KeyModifiers::CONTROL);
        assert!(is_quit_key(key));
    }

    #[test]
    fn test_is_quit_key_q() {
        let key = KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE);
        assert!(is_quit_key(key));
    }

    #[test]
    fn test_is_quit_key_esc() {
        let key = KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE);
        assert!(is_quit_key(key));
    }

    #[test]
    fn test_is_refresh_key() {
        let key = KeyEvent::new(KeyCode::Char('r'), KeyModifiers::CONTROL);
        assert!(is_refresh_key(key));
    }

    #[test]
    fn test_is_help_key() {
        let key_question = KeyEvent::new(KeyCode::Char('?'), KeyModifiers::NONE);
        assert!(is_help_key(key_question));

        let key_h = KeyEvent::new(KeyCode::Char('h'), KeyModifiers::NONE);
        assert!(is_help_key(key_h));

        let key_f1 = KeyEvent::new(KeyCode::F(1), KeyModifiers::NONE);
        assert!(is_help_key(key_f1));
    }

    #[test]
    fn test_event_handler_creation() {
        let _handler = EventHandler::new();
        // Handler should be created without errors
    }
}
