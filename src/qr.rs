mod image;
mod print;

pub use crate::qr::image::{Dot, Grid};
pub use crate::qr::print::{Colors, Output, Print};
use qrcode::types::QrError;
pub use qrcode::QrCode;
use std::io::Error as IoError;
pub use termcolor::Color;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum PrintError {
    #[error("error generating QR code: {0}")]
    Qr(#[from] QrError),

    #[error("error printing QR code: {0}")]
    Io(#[from] IoError),
}

pub fn try_print(data: impl AsRef<[u8]>) -> Result<(), PrintError> {
    Ok(QrCode::new(data)?
        .render::<Dot>()
        .build()
        .print(Output::default(), Colors::from_env())?)
}

pub fn print_with_url(url: impl AsRef<str>) {
    let url = url.as_ref();
    println!("{url}");
    print(url);
}

pub fn print(data: impl AsRef<[u8]>) {
    try_print(data).expect("unhandled error printing QR code to terminal")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn print_qr_without_panic() {
        // NOTE: Use `cargo test --features qr -- --nocapture` for a beautiful
        //   terminal QR code graphic. The `cargo test` command line arguments
        //   are manually checked as `termcolor` ignores `Stdout` capturing.
        if env::args().any(|arg| arg == "--nocapture") {
            print("wc:8a5e5bdc-a0e4-4702-ba63-8f1a5655744f@1?bridge=https%3A%2F%2Fbridge.walletconnect.org&key=41791102999c339c844880b23950704cc43aa840f3739e365323cda4dfa89e7a");
        }
    }
}
