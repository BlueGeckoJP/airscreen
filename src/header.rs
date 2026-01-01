pub struct Header {
    pub mode: u8,
    pub width: u32,
    pub height: u32,
    pub total_len: u32,
    pub payload_len: u32,
}

impl From<&[u8; 17]> for Header {
    fn from(bytes: &[u8; 17]) -> Self {
        let mode = bytes[0];
        let width = u32::from_le_bytes([bytes[1], bytes[2], bytes[3], bytes[4]]);
        let height = u32::from_le_bytes([bytes[5], bytes[6], bytes[7], bytes[8]]);
        let total_len = u32::from_le_bytes([bytes[9], bytes[10], bytes[11], bytes[12]]);
        let payload_len = u32::from_le_bytes([bytes[13], bytes[14], bytes[15], bytes[16]]);

        Header {
            mode,
            width,
            height,
            total_len,
            payload_len,
        }
    }
}

impl From<Header> for [u8; 17] {
    fn from(header: Header) -> Self {
        let mut bytes = [0u8; 17];
        bytes[0] = header.mode;
        bytes[1..5].copy_from_slice(&header.width.to_le_bytes());
        bytes[5..9].copy_from_slice(&header.height.to_le_bytes());
        bytes[9..13].copy_from_slice(&header.total_len.to_le_bytes());
        bytes[13..17].copy_from_slice(&header.payload_len.to_le_bytes());
        bytes
    }
}
