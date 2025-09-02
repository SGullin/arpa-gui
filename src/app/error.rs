use arpa::ARPAError;

pub enum AppError {
    ARPA(ARPAError),
}
