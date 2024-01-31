use hex;

pub struct HexParser;

impl HexParser {
    pub(crate) fn parse_hex_string(hex_string: &str) -> Result<Vec<u8>, hex::FromHexError> {
        if hex_string.starts_with("0x") {
            hex::decode(&hex_string[2..])
        } else {
            hex::decode(hex_string)
        }
    }
}
