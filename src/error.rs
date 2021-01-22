#[derive(Debug)]
pub enum Error {
    RequestFailed(attohttpc::Error),
    CsvParseError(csv::Error),
    JSONParseError(serde_json::Error),
    NotFound
}

impl From<attohttpc::Error> for Error {
    fn from(e: attohttpc::Error) -> Self {
        Self::RequestFailed(e)
    }
}

impl From<csv::Error> for Error {
    fn from(e: csv::Error) -> Self {
        Self::CsvParseError(e)
    }
}

impl From<serde_json::Error> for Error {
    fn from(e: serde_json::Error) -> Self {
        Self::JSONParseError(e)
    }
}