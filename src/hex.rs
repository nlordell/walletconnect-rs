use data_encoding::{DecodeError, DecodeKind, HEXLOWER_PERMISSIVE as HEX};

pub fn encode(data: impl AsRef<[u8]>) -> String {
    HEX.encode(data.as_ref())
}

pub fn decode_mut(
    bytes: impl AsRef<[u8]>,
    mut buffer: impl AsMut<[u8]>,
) -> Result<(), DecodeError> {
    let bytes = bytes.as_ref();
    let buffer = buffer.as_mut();

    let decode_len = HEX.decode_len(bytes.len())?;
    if buffer.len() != decode_len {
        return Err(DecodeError {
            position: 0,
            kind: DecodeKind::Length,
        });
    }

    HEX.decode_mut(bytes, buffer).map_err(|err| err.error)?;
    Ok(())
}

pub fn decode(bytes: impl AsRef<[u8]>) -> Result<Vec<u8>, DecodeError> {
    let bytes = bytes.as_ref();

    let buffer_len = HEX.decode_len(bytes.len())?;
    let mut buffer = vec![0; buffer_len];

    decode_mut(bytes, &mut buffer)?;
    Ok(buffer)
}
