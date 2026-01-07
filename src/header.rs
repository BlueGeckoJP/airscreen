pub const HEADER_SIZE: usize = 12;

pub struct Header {
    pub width: u32,
    pub height: u32,
    pub payload_len: u32,
}

impl From<&[u8; HEADER_SIZE]> for Header {
    fn from(bytes: &[u8; HEADER_SIZE]) -> Self {
        let width = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
        let height = u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]);
        let payload_len = u32::from_le_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]);

        Header {
            width,
            height,
            payload_len,
        }
    }
}

impl From<Header> for [u8; HEADER_SIZE] {
    fn from(header: Header) -> Self {
        let mut bytes = [0u8; HEADER_SIZE];
        bytes[0..4].copy_from_slice(&header.width.to_le_bytes());
        bytes[4..8].copy_from_slice(&header.height.to_le_bytes());
        bytes[8..12].copy_from_slice(&header.payload_len.to_le_bytes());
        bytes
    }
}
