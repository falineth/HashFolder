use std::time::Duration;

use crossterm::event::{self, Event, KeyCode, KeyEvent};

use crate::errors::{AppError, AppErrorResult};

pub fn check_exit_key_pressed() -> Result<(), AppError> {
    loop {
        if event::poll(Duration::ZERO).app_err()? {
            match event::read().app_err()? {
                Event::Key(KeyEvent {
                    code,
                    modifiers: _,
                    kind: _,
                    state: _,
                }) => match code {
                    KeyCode::Char('q') => {
                        Err(AppError::new(format!("Abort key pressed")))?;
                    }
                    _ => (),
                },
                _ => (),
            }
        } else {
            return Ok(());
        }
    }
}
