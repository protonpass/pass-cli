use std::error::Error;

pub trait MuonErrorExt {
    fn is_logged_out_error(&self) -> bool;
}

impl MuonErrorExt for muon::Error {
    fn is_logged_out_error(&self) -> bool {
        if self.kind() == muon::ErrorKind::Send
            && let Some(source) = self.source()
        {
            return source.to_string() == "non-existent session";
        }

        false
    }
}
