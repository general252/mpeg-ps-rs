use crate::bitstream::{BitStream, BitStreamWriter};
use crate::error::PSError;
use crate::util::calc_crc32;

pub const PS_STREAM_UNKNOW: u8 = 0xFF;
pub const PS_STREAM_AAC: u8 = 0x0F;
pub const PS_STREAM_H264: u8 = 0x1B;
pub const PS_STREAM_H265: u8 = 0x24;
pub const PS_STREAM_G711A: u8 = 0x90;
pub const PS_STREAM_G711U: u8 = 0x91;

pub const PES_STREAM_END: u8 = 0xB9;
pub const PES_STREAM_START: u8 = 0xBA;
pub const PES_STREAM_SYSTEM_HEAD: u8 = 0xBB;
pub const PES_STREAM_MAP: u8 = 0xBC;
pub const PES_STREAM_PRIVATE: u8 = 0xBD;
pub const PES_STREAM_AUDIO: u8 = 0xC0;
pub const PES_STREAM_VIDEO: u8 = 0xE0;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PSPackHeader {
    pub is_mpeg1: bool,
    pub system_clock_reference_base: u64,
    pub system_clock_reference_extension: u16,
    pub program_mux_rate: u32,
    pub pack_stuffing_length: u8,
}

impl PSPackHeader {
    pub fn new() -> Self {
        Self {
            is_mpeg1: false,
            system_clock_reference_base: 0,
            system_clock_reference_extension: 0,
            program_mux_rate: 0,
            pack_stuffing_length: 0,
        }
    }

    pub fn encode(&self, bsw: &mut BitStreamWriter) {
        bsw.put_bytes(&[0x00, 0x00, 0x01, 0xBA]);
        bsw.put_uint8(1, 2);
        bsw.put_uint64(self.system_clock_reference_base >> 30, 3);
        bsw.put_uint8(1, 1);
        bsw.put_uint64(self.system_clock_reference_base >> 15, 15);
        bsw.put_uint8(1, 1);
        bsw.put_uint64(self.system_clock_reference_base, 15);
        bsw.put_uint8(1, 1);
        bsw.put_uint16(self.system_clock_reference_extension, 9);
        bsw.put_uint8(1, 1);
        bsw.put_uint32(self.program_mux_rate, 22);
        bsw.put_uint8(1, 1);
        bsw.put_uint8(1, 1);
        bsw.put_uint8(0x1F, 5);
        bsw.put_uint8(self.pack_stuffing_length, 3);
        bsw.put_repeat_value(0xFF, self.pack_stuffing_length as usize);
    }
}

impl Default for PSPackHeader {
    fn default() -> Self {
        Self::new()
    }
}

pub trait Display {
    fn pretty_print(&self);
}

impl PSPackHeader {
    pub fn decode(&mut self, bs: &mut BitStream) -> Result<(), PSError> {
        if bs.remain_bytes() < 5 {
            return Err(PSError::NeedMore);
        }
        let prefix = bs.get_bits(32) as u32;
        if prefix != 0x000001BA {
            return Err(PSError::ParserError);
        }
        if bs.next_bits(2) == 0x01 {
            if bs.remain_bytes() < 10 {
                return Err(PSError::NeedMore);
            }
            self.decode_mpeg2(bs)?;
        } else if bs.next_bits(4) == 0x02 {
            if bs.remain_bytes() < 8 {
                return Err(PSError::NeedMore);
            }
            self.is_mpeg1 = true;
            self.decode_mpeg1(bs)?;
        } else {
            return Err(PSError::ParserError);
        }
        Ok(())
    }

    fn decode_mpeg2(&mut self, bs: &mut BitStream) -> Result<(), PSError> {
        bs.skip_bits(2);
        self.system_clock_reference_base = bs.get_bits(3);
        bs.skip_bits(1);
        self.system_clock_reference_base = (self.system_clock_reference_base << 15) | bs.get_bits(15);
        bs.skip_bits(1);
        self.system_clock_reference_base = (self.system_clock_reference_base << 15) | bs.get_bits(15);
        bs.skip_bits(1);
        self.system_clock_reference_extension = bs.get_bits(9) as u16;
        bs.skip_bits(1);
        self.program_mux_rate = bs.get_bits(22) as u32;
        bs.skip_bits(1);
        bs.skip_bits(1);
        bs.skip_bits(5);
        self.pack_stuffing_length = bs.get_bits(3) as u8;
        if bs.remain_bytes() < self.pack_stuffing_length as usize {
            bs.unread(10 * 8);
            return Err(PSError::NeedMore);
        }
        bs.skip_bits(self.pack_stuffing_length as usize * 8);
        Ok(())
    }

    fn decode_mpeg1(&mut self, bs: &mut BitStream) -> Result<(), PSError> {
        bs.skip_bits(4);
        self.system_clock_reference_base = bs.get_bits(3);
        bs.skip_bits(1);
        self.system_clock_reference_base = (self.system_clock_reference_base << 15) | bs.get_bits(15);
        bs.skip_bits(1);
        self.system_clock_reference_base = (self.system_clock_reference_base << 15) | bs.get_bits(15);
        bs.skip_bits(1);
        self.system_clock_reference_extension = 1;
        self.program_mux_rate = bs.get_bits(7) as u32;
        bs.skip_bits(1);
        self.program_mux_rate = (self.program_mux_rate << 15) | bs.get_bits(15) as u32;
        bs.skip_bits(1);
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ElementaryStream {
    pub stream_id: u8,
    pub p_std_buffer_bound_scale: u8,
    pub p_std_buffer_size_bound: u16,
}

impl ElementaryStream {
    pub fn new(sid: u8) -> Self {
        Self {
            stream_id: sid,
            p_std_buffer_bound_scale: 0,
            p_std_buffer_size_bound: 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SystemHeader {
    pub header_length: u16,
    pub rate_bound: u32,
    pub audio_bound: u8,
    pub fixed_flag: u8,
    pub csps_flag: u8,
    pub system_audio_lock_flag: u8,
    pub system_video_lock_flag: u8,
    pub video_bound: u8,
    pub packet_rate_restriction_flag: u8,
    pub streams: Vec<ElementaryStream>,
}

impl SystemHeader {
    pub fn new() -> Self {
        Self {
            header_length: 0,
            rate_bound: 26234,
            audio_bound: 0,
            fixed_flag: 0,
            csps_flag: 0,
            system_audio_lock_flag: 0,
            system_video_lock_flag: 0,
            video_bound: 0,
            packet_rate_restriction_flag: 0,
            streams: Vec::new(),
        }
    }

    pub fn encode(&self, bsw: &mut BitStreamWriter) {
        bsw.put_bytes(&[0x00, 0x00, 0x01, 0xBB]);
        let loc = bsw.byte_offset();
        bsw.put_uint16(0, 16);
        bsw.mark();
        bsw.put_uint8(1, 1);
        bsw.put_uint32(self.rate_bound, 22);
        bsw.put_uint8(1, 1);
        bsw.put_uint8(self.audio_bound, 6);
        bsw.put_uint8(self.fixed_flag, 1);
        bsw.put_uint8(self.csps_flag, 1);
        bsw.put_uint8(self.system_audio_lock_flag, 1);
        bsw.put_uint8(self.system_video_lock_flag, 1);
        bsw.put_uint8(1, 1);
        bsw.put_uint8(self.video_bound, 5);
        bsw.put_uint8(self.packet_rate_restriction_flag, 1);
        bsw.put_uint8(0x7F, 7);
        for stream in &self.streams {
            bsw.put_uint8(stream.stream_id, 8);
            bsw.put_uint8(3, 2);
            bsw.put_uint8(stream.p_std_buffer_bound_scale, 1);
            bsw.put_uint16(stream.p_std_buffer_size_bound, 13);
        }
        let length = bsw.distance_from_mark() / 8;
        bsw.set_uint16(length as u16, loc);
    }
}

impl Default for SystemHeader {
    fn default() -> Self {
        Self::new()
    }
}

impl SystemHeader {
    pub fn decode(&mut self, bs: &mut BitStream) -> Result<(), PSError> {
        if bs.remain_bytes() < 12 {
            return Err(PSError::NeedMore);
        }
        let prefix = bs.get_bits(32) as u32;
        if prefix != 0x000001BB {
            return Err(PSError::ParserError);
        }
        self.header_length = bs.get_bits(16) as u16;
        if bs.remain_bytes() < self.header_length as usize {
            bs.unread(6 * 8);
            return Err(PSError::NeedMore);
        }
        if self.header_length < 6 || (self.header_length - 6) % 3 != 0 {
            return Err(PSError::ParserError);
        }
        bs.skip_bits(1);
        self.rate_bound = bs.get_bits(22) as u32;
        bs.skip_bits(1);
        self.audio_bound = bs.get_bits(6) as u8;
        self.fixed_flag = bs.get_bits(1) as u8;
        self.csps_flag = bs.get_bits(1) as u8;
        self.system_audio_lock_flag = bs.get_bits(1) as u8;
        self.system_video_lock_flag = bs.get_bits(1) as u8;
        bs.skip_bits(1);
        self.video_bound = bs.get_bits(5) as u8;
        self.packet_rate_restriction_flag = bs.get_bits(1) as u8;
        bs.skip_bits(7);
        self.streams.clear();
        let mut least = self.header_length - 6;
        while least > 0 && bs.next_bits(1) == 0x01 {
            let mut es = ElementaryStream::new(0);
            es.stream_id = bs.get_bits(8) as u8;
            bs.skip_bits(2);
            es.p_std_buffer_bound_scale = bs.get_bit();
            es.p_std_buffer_size_bound = bs.get_bits(13) as u16;
            self.streams.push(es);
            least -= 3;
        }
        if least > 0 {
            return Err(PSError::ParserError);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ElementaryStreamElem {
    pub stream_type: u8,
    pub elementary_stream_id: u8,
    pub elementary_stream_info_length: u16,
}

impl ElementaryStreamElem {
    pub fn new(stype: u8, esid: u8) -> Self {
        Self {
            stream_type: stype,
            elementary_stream_id: esid,
            elementary_stream_info_length: 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProgramStreamMap {
    pub map_stream_id: u8,
    pub program_stream_map_length: u16,
    pub current_next_indicator: u8,
    pub program_stream_map_version: u8,
    pub program_stream_info_length: u16,
    pub elementary_stream_map_length: u16,
    pub stream_map: Vec<ElementaryStreamElem>,
}

impl ProgramStreamMap {
    pub fn new() -> Self {
        Self {
            map_stream_id: 0,
            program_stream_map_length: 0,
            current_next_indicator: 1,
            program_stream_map_version: 1,
            program_stream_info_length: 0,
            elementary_stream_map_length: 0,
            stream_map: Vec::new(),
        }
    }

    pub fn encode(&self, bsw: &mut BitStreamWriter) {
        bsw.put_bytes(&[0x00, 0x00, 0x01, 0xBC]);
        let loc = bsw.byte_offset();
        bsw.put_uint16(self.program_stream_map_length, 16);
        bsw.mark();
        bsw.put_uint8(self.current_next_indicator, 1);
        bsw.put_uint8(3, 2);
        bsw.put_uint8(self.program_stream_map_version, 5);
        bsw.put_uint8(0x7F, 7);
        bsw.put_uint8(1, 1);
        bsw.put_uint16(0, 16);
        let es_map_length = (self.stream_map.len() * 4) as u16;
        bsw.put_uint16(es_map_length, 16);
        for streaminfo in &self.stream_map {
            bsw.put_uint8(streaminfo.stream_type, 8);
            bsw.put_uint8(streaminfo.elementary_stream_id, 8);
            bsw.put_uint16(0, 16);
        }
        let length = bsw.distance_from_mark() / 8 + 4;
        bsw.set_uint16(length as u16, loc);
        let start = bsw.byte_offset() - length as usize;
        let data = bsw.bits()[start..bsw.byte_offset()].to_vec();
        let crc = calc_crc32(0xFFFFFFFF, &data);
        bsw.put_uint32(crc, 32);
    }
}

impl Default for ProgramStreamMap {
    fn default() -> Self {
        Self::new()
    }
}

impl ProgramStreamMap {
    pub fn decode(&mut self, bs: &mut BitStream) -> Result<(), PSError> {
        if bs.remain_bytes() < 16 {
            return Err(PSError::NeedMore);
        }
        let prefix = bs.get_bits(24) as u32;
        if prefix != 0x000001 {
            return Err(PSError::ParserError);
        }
        self.map_stream_id = bs.get_bits(8) as u8;
        if self.map_stream_id != 0xBC {
            return Err(PSError::ParserError);
        }
        self.program_stream_map_length = bs.get_bits(16) as u16;
        if bs.remain_bytes() < self.program_stream_map_length as usize {
            bs.unread(6 * 8);
            return Err(PSError::NeedMore);
        }
        self.current_next_indicator = bs.get_bits(1) as u8;
        bs.skip_bits(2);
        self.program_stream_map_version = bs.get_bits(5) as u8;
        bs.skip_bits(8);
        self.program_stream_info_length = bs.get_bits(16) as u16;
        if bs.remain_bytes() < self.program_stream_info_length as usize + 2 {
            bs.unread(10 * 8);
            return Err(PSError::NeedMore);
        }
        bs.skip_bits(self.program_stream_info_length as usize * 8);
        self.elementary_stream_map_length = bs.get_bits(16) as u16;
        if self.program_stream_map_length != 6 + self.program_stream_info_length + self.elementary_stream_map_length + 4 {
            return Err(PSError::ParserError);
        }
        if bs.remain_bytes() < self.elementary_stream_map_length as usize + 2 {
            bs.unread(12 * 8 + self.program_stream_info_length as usize * 8);
            return Err(PSError::NeedMore);
        }
        let mut i: usize = 0;
        self.stream_map.clear();
        while i < self.elementary_stream_map_length as usize {
            let mut elem = ElementaryStreamElem::new(0, 0);
            elem.stream_type = bs.get_bits(8) as u8;
            elem.elementary_stream_id = bs.get_bits(8) as u8;
            elem.elementary_stream_info_length = bs.get_bits(16) as u16;
            if bs.remain_bytes() < elem.elementary_stream_info_length as usize {
                return Err(PSError::ParserError);
            }
            bs.skip_bits(elem.elementary_stream_info_length as usize * 8);
            i += 4 + elem.elementary_stream_info_length as usize;
            self.stream_map.push(elem);
        }
        if i != self.elementary_stream_map_length as usize {
            return Err(PSError::ParserError);
        }
        bs.skip_bits(32);
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProgramStreamDirectory {
    pub pes_packet_length: u16,
}

impl ProgramStreamDirectory {
    pub fn new() -> Self {
        Self { pes_packet_length: 0 }
    }

    pub fn decode(&mut self, bs: &mut BitStream) -> Result<(), PSError> {
        if bs.remain_bytes() < 6 {
            return Err(PSError::NeedMore);
        }
        let prefix = bs.get_bits(32) as u32;
        if prefix != 0x000001FF {
            return Err(PSError::ParserError);
        }
        self.pes_packet_length = bs.get_bits(16) as u16;
        if bs.remain_bytes() < self.pes_packet_length as usize {
            bs.unread(6 * 8);
            return Err(PSError::NeedMore);
        }
        bs.skip_bits(self.pes_packet_length as usize * 8);
        Ok(())
    }
}

impl Default for ProgramStreamDirectory {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommonPesPacket {
    pub stream_id: u8,
    pub pes_packet_length: u16,
}

impl CommonPesPacket {
    pub fn new() -> Self {
        Self {
            stream_id: 0,
            pes_packet_length: 0,
        }
    }

    pub fn decode(&mut self, bs: &mut BitStream) -> Result<(), PSError> {
        if bs.remain_bytes() < 6 {
            return Err(PSError::NeedMore);
        }
        bs.skip_bits(24);
        self.stream_id = bs.get_bits(8) as u8;
        self.pes_packet_length = bs.get_bits(16) as u16;
        if bs.remain_bytes() < self.pes_packet_length as usize {
            bs.unread(6 * 8);
            return Err(PSError::NeedMore);
        }
        bs.skip_bits(self.pes_packet_length as usize * 8);
        Ok(())
    }
}

impl Default for CommonPesPacket {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Default)]
pub struct PSPacket {
    pub header: Option<PSPackHeader>,
    pub system: Option<SystemHeader>,
    pub psm: Option<ProgramStreamMap>,
    pub psd: Option<ProgramStreamDirectory>,
    pub common_pes: Option<CommonPesPacket>,
    pub pes: Option<crate::pes_proto::PesPacket>,
}

impl PSPacket {
    pub fn new() -> Self {
        Self {
            header: None,
            system: None,
            psm: None,
            psd: None,
            common_pes: None,
            pes: None,
        }
    }

    pub fn header(&self) -> Option<&PSPackHeader> {
        self.header.as_ref()
    }
    pub fn system(&self) -> Option<&SystemHeader> {
        self.system.as_ref()
    }
    pub fn psm(&self) -> Option<&ProgramStreamMap> {
        self.psm.as_ref()
    }
}
