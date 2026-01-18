use std::time::Duration;

use crossterm::event::{self, Event, KeyCode, KeyEvent};

use crate::errors::{AppError, AppErrorResult};

pub fn check_exit_key_pressed() -> Result<(), AppError> {
    loop {
        if event::poll(Duration::ZERO).app_err()? {
            if let Event::Key(KeyEvent {
                code,
                modifiers: _,
                kind: _,
                state: _,
            }) = event::read().app_err()?
                && let KeyCode::Char('q') = code
            {
                Err(AppError::new("Abort key pressed".into()))?;
            }
        } else {
            return Ok(());
        }
    }
}
