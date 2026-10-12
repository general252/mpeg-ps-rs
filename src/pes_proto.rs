use crate::bitstream::{BitStream, BitStreamWriter};
use crate::error::PSError;

pub const H264_AUD_NALU: [u8; 6] = [0x00, 0x00, 0x00, 0x01, 0x09, 0xF0];
pub const H265_AUD_NALU: [u8; 7] = [0x00, 0x00, 0x00, 0x01, 0x46, 0x01, 0x50];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PesPacket {
    pub stream_id: u8,
    pub pes_packet_length: u16,
    pub pes_scrambling_control: u8,
    pub pes_priority: u8,
    pub data_alignment_indicator: u8,
    pub copyright: u8,
    pub original_or_copy: u8,
    pub pts_dts_flags: u8,
    pub escr_flag: u8,
    pub es_rate_flag: u8,
    pub dsm_trick_mode_flag: u8,
    pub additional_copy_info_flag: u8,
    pub pes_crc_flag: u8,
    pub pes_extension_flag: u8,
    pub pes_header_data_length: u8,
    pub pts: u64,
    pub dts: u64,
    pub escr_base: u64,
    pub escr_extension: u16,
    pub es_rate: u32,
    pub trick_mode_control: u8,
    pub trick_value: u8,
    pub additional_copy_info: u8,
    pub previous_pes_packet_crc: u16,
    pub pes_payload: Vec<u8>,
}

impl PesPacket {
    pub fn new() -> Self {
        Self {
            stream_id: 0,
            pes_packet_length: 0,
            pes_scrambling_control: 0,
            pes_priority: 0,
            data_alignment_indicator: 0,
            copyright: 0,
            original_or_copy: 0,
            pts_dts_flags: 0,
            escr_flag: 0,
            es_rate_flag: 0,
            dsm_trick_mode_flag: 0,
            additional_copy_info_flag: 0,
            pes_crc_flag: 0,
            pes_extension_flag: 0,
            pes_header_data_length: 0,
            pts: 0,
            dts: 0,
            escr_base: 0,
            escr_extension: 0,
            es_rate: 0,
            trick_mode_control: 0,
            trick_value: 0,
            additional_copy_info: 0,
            previous_pes_packet_crc: 0,
            pes_payload: Vec::new(),
        }
    }

    pub fn encode(&self, bsw: &mut BitStreamWriter) {
        bsw.put_bytes(&[0x00, 0x00, 0x01]);
        bsw.put_byte(self.stream_id);
        bsw.put_uint16(self.pes_packet_length, 16);
        bsw.put_uint8(0x02, 2);
        bsw.put_uint8(self.pes_scrambling_control, 2);
        bsw.put_uint8(self.pes_priority, 1);
        bsw.put_uint8(self.data_alignment_indicator, 1);
        bsw.put_uint8(self.copyright, 1);
        bsw.put_uint8(self.original_or_copy, 1);
        bsw.put_uint8(self.pts_dts_flags, 2);
        bsw.put_uint8(self.escr_flag, 1);
        bsw.put_uint8(self.es_rate_flag, 1);
        bsw.put_uint8(self.dsm_trick_mode_flag, 1);
        bsw.put_uint8(self.additional_copy_info_flag, 1);
        bsw.put_uint8(self.pes_crc_flag, 1);
        bsw.put_uint8(self.pes_extension_flag, 1);
        bsw.put_byte(self.pes_header_data_length);

        if self.pts_dts_flags == 0x02 {
            bsw.put_uint8(0x02, 4);
            bsw.put_uint64(self.pts >> 30, 3);
            bsw.put_uint8(0x01, 1);
            bsw.put_uint64(self.pts >> 15, 15);
            bsw.put_uint8(0x01, 1);
            bsw.put_uint64(self.pts, 15);
            bsw.put_uint8(0x01, 1);
        }

        if self.pts_dts_flags == 0x03 {
            bsw.put_uint8(0x03, 4);
            bsw.put_uint64(self.pts >> 30, 3);
            bsw.put_uint8(0x01, 1);
            bsw.put_uint64(self.pts >> 15, 15);
            bsw.put_uint8(0x01, 1);
            bsw.put_uint64(self.pts, 15);
            bsw.put_uint8(0x01, 1);

            bsw.put_uint8(0x01, 4);
            bsw.put_uint64(self.dts >> 30, 3);
            bsw.put_uint8(0x01, 1);
            bsw.put_uint64(self.dts >> 15, 15);
            bsw.put_uint8(0x01, 1);
            bsw.put_uint64(self.dts, 15);
            bsw.put_uint8(0x01, 1);
        }

        if self.escr_flag == 1 {
            bsw.put_uint8(0x03, 2);
            bsw.put_uint64(self.escr_base >> 30, 3);
            bsw.put_uint8(0x01, 1);
            bsw.put_uint64(self.escr_base >> 15, 15);
            bsw.put_uint8(0x01, 1);
            bsw.put_uint64(self.escr_base, 15);
            bsw.put_uint8(0x01, 1);
        }

        bsw.put_bytes(&self.pes_payload);
    }

    pub fn clear_payload(&mut self) {
        self.pes_payload.clear();
    }

    pub fn decode(&mut self, bs: &mut BitStream) -> Result<(), PSError> {
        if bs.remain_bytes() < 9 {
            return Err(PSError::NeedMore);
        }
        bs.skip_bits(24);
        self.stream_id = bs.get_bits(8) as u8;
        self.pes_packet_length = bs.get_bits(16) as u16;
        bs.skip_bits(2);
        self.pes_scrambling_control = bs.get_bits(2) as u8;
        self.pes_priority = bs.get_bits(1) as u8;
        self.data_alignment_indicator = bs.get_bits(1) as u8;
        self.copyright = bs.get_bits(1) as u8;
        self.original_or_copy = bs.get_bits(1) as u8;
        self.pts_dts_flags = bs.get_bits(2) as u8;
        self.escr_flag = bs.get_bits(1) as u8;
        self.es_rate_flag = bs.get_bits(1) as u8;
        self.dsm_trick_mode_flag = bs.get_bits(1) as u8;
        self.additional_copy_info_flag = bs.get_bits(1) as u8;
        self.pes_crc_flag = bs.get_bits(1) as u8;
        self.pes_extension_flag = bs.get_bits(1) as u8;
        self.pes_header_data_length = bs.get_bits(8) as u8;
        if bs.remain_bytes() < self.pes_header_data_length as usize {
            bs.unread(9 * 8);
            return Err(PSError::NeedMore);
        }
        bs.mark();
        if self.pts_dts_flags & 0x02 == 0x02 {
            bs.skip_bits(4);
            self.pts = bs.get_bits(3);
            bs.skip_bits(1);
            self.pts = (self.pts << 15) | bs.get_bits(15);
            bs.skip_bits(1);
            self.pts = (self.pts << 15) | bs.get_bits(15);
            bs.skip_bits(1);
        }
        if self.pts_dts_flags & 0x03 == 0x03 {
            bs.skip_bits(4);
            self.dts = bs.get_bits(3);
            bs.skip_bits(1);
            self.dts = (self.dts << 15) | bs.get_bits(15);
            bs.skip_bits(1);
            self.dts = (self.dts << 15) | bs.get_bits(15);
            bs.skip_bits(1);
        } else {
            self.dts = self.pts;
        }
        if self.escr_flag == 1 {
            bs.skip_bits(2);
            self.escr_base = bs.get_bits(3);
            bs.skip_bits(1);
            self.escr_base = (self.escr_base << 15) | bs.get_bits(15);
            bs.skip_bits(1);
            self.escr_base = (self.escr_base << 15) | bs.get_bits(15);
            bs.skip_bits(1);
            self.escr_extension = bs.get_bits(9) as u16;
            bs.skip_bits(1);
        }
        if self.es_rate_flag == 1 {
            bs.skip_bits(1);
            self.es_rate = bs.get_bits(22) as u32;
            bs.skip_bits(1);
        }
        if self.dsm_trick_mode_flag == 1 {
            self.trick_mode_control = bs.get_bits(3) as u8;
            self.trick_value = bs.get_bits(5) as u8;
        }
        if self.additional_copy_info_flag == 1 {
            self.additional_copy_info = bs.get_bits(7) as u8;
        }
        if self.pes_crc_flag == 1 {
            self.previous_pes_packet_crc = bs.get_bits(16) as u16;
        }
        let loc = bs.distance_from_mark();
        let skip_remain = self.pes_header_data_length as usize * 8 - loc;
        if skip_remain > 0 {
            bs.skip_bits(skip_remain);
        }
        let data_len = (self.pes_packet_length as usize).saturating_sub(3 + self.pes_header_data_length as usize);
        if bs.remain_bytes() < data_len {
            self.pes_payload = bs.remain_data().to_vec();
            bs.unread((9 + self.pes_header_data_length as usize) * 8);
            return Err(PSError::NeedMore);
        }
        if self.pes_packet_length == 0 || bs.remain_bits() <= data_len * 8 {
            self.pes_payload = bs.remain_data().to_vec();
            let remaining = bs.remain_bits();
            if remaining > 0 {
                bs.skip_bits(remaining);
            }
        } else {
            self.pes_payload = bs.remain_data()[..data_len].to_vec();
            bs.skip_bits(data_len * 8);
        }
        Ok(())
    }

    pub fn decode_mpeg1(&mut self, bs: &mut BitStream) -> Result<(), PSError> {
        if bs.remain_bytes() < 6 {
            return Err(PSError::NeedMore);
        }
        bs.skip_bits(24);
        self.stream_id = bs.get_bits(8) as u8;
        self.pes_packet_length = bs.get_bits(16) as u16;
        if self.pes_packet_length != 0 && bs.remain_bytes() < self.pes_packet_length as usize {
            bs.unread(6 * 8);
            return Err(PSError::NeedMore);
        }
        bs.mark();
        while bs.next_bits(8) == 0xFF {
            bs.skip_bits(8);
        }
        if bs.next_bits(2) == 0x01 {
            bs.skip_bits(16);
        }
        if bs.next_bits(4) == 0x02 {
            bs.skip_bits(4);
            self.pts = bs.get_bits(3);
            bs.skip_bits(1);
            self.pts = (self.pts << 15) | bs.get_bits(15);
            bs.skip_bits(1);
            self.pts = (self.pts << 15) | bs.get_bits(15);
            bs.skip_bits(1);
        } else if bs.next_bits(4) == 0x03 {
            bs.skip_bits(4);
            self.pts = bs.get_bits(3);
            bs.skip_bits(1);
            self.pts = (self.pts << 15) | bs.get_bits(15);
            bs.skip_bits(1);
            self.pts = (self.pts << 15) | bs.get_bits(15);
            bs.skip_bits(1);
            self.dts = bs.get_bits(3);
            bs.skip_bits(1);
            self.dts = (self.dts << 15) | bs.get_bits(15);
            bs.skip_bits(1);
            self.dts = (self.dts << 15) | bs.get_bits(15);
            bs.skip_bits(1);
        } else if bs.next_bits(8) == 0x0F {
            bs.skip_bits(8);
        } else {
            return Err(PSError::ParserError);
        }
        let loc = bs.distance_from_mark() / 8;
        if self.pes_packet_length < loc as u16 {
            return Err(PSError::ParserError);
        }
        if self.pes_packet_length == 0 || bs.remain_bits() <= (self.pes_packet_length as usize - loc) * 8 {
            self.pes_payload = bs.remain_data().to_vec();
            let remaining = bs.remain_bits();
            if remaining > 0 {
                bs.skip_bits(remaining);
            }
        } else {
            let payload_len = self.pes_packet_length as usize - loc;
            self.pes_payload = bs.remain_data()[..payload_len].to_vec();
            bs.skip_bits(payload_len * 8);
        }
        Ok(())
    }
}

impl Default for PesPacket {
    fn default() -> Self {
        Self::new()
    }
}
