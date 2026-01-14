use std::error::Error;
use std::fmt::Display;

#[derive(Debug, Default)]
pub struct AbortError {
    message: String,
}

impl Display for AbortError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

#[derive(Debug)]
pub struct CaughtError {
    pub caller: String,
    pub error: Box<dyn Error>,
}

impl Display for CaughtError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}\n{}", self.caller, self.error)
    }
}

impl<T> From<Box<T>> for CaughtError
where
    T: Error + 'static,
{
    #[track_caller]
    fn from(error: Box<T>) -> Self {
        let loc = std::panic::Location::caller();

        CaughtError {
            caller: format!("Error at {}:{}", loc.file(), loc.line()),
            error,
        }
    }
}

pub trait AppErrorResult<T> {
    fn app_err(self) -> Result<T, AppError>;
}

impl<T1, T2> AppErrorResult<T1> for Result<T1, T2>
where
    T2: Error + 'static,
{
    #[track_caller]
    fn app_err(self) -> Result<T1, AppError> {
        let loc = std::panic::Location::caller();

        match self {
            Ok(value) => Ok(value),
            Err(err) => Err(AppError::Caught(CaughtError {
                caller: format!("Error at {}:{}", loc.file(), loc.line()),
                error: Box::new(err),
            })),
        }
    }
}

#[derive(Debug)]
pub enum AppError {
    Caught(CaughtError),
    Abort(AbortError),
}

impl Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AppError::Abort(abort) => write!(f, "{}", abort.message),
            AppError::Caught(caught) => write!(f, "{} {:?}", caught.caller, caught.error),
        }
    }
}

impl AppError {
    pub fn new(message: String) -> AppError {
        return AppError::Abort(AbortError { message });
    }
}
