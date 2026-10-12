use std::collections::HashMap;

use crate::bitstream::BitStream;
use crate::codec::{H264NalType, H265NalType};
use crate::error::PSError;
use crate::pes_proto::PesPacket;
use crate::ps_proto::{
    CommonPesPacket, Display, PES_STREAM_AUDIO, PES_STREAM_VIDEO, PSPackHeader, PSPacket,
    ProgramStreamDirectory, ProgramStreamMap, SystemHeader, PS_STREAM_AAC, PS_STREAM_G711A,
    PS_STREAM_G711U, PS_STREAM_H264, PS_STREAM_H265, PS_STREAM_UNKNOW,
};
use crate::util::{
    find_start_code, h264_nalu_type, h264_nalu_type_without_start_code, h265_nalu_type,
    h265_nalu_type_without_start_code, split_frame,
};

struct PSStream {
    sid: u8,
    cid: u8,
    pts: u64,
    dts: u64,
    stream_buf: Vec<u8>,
}

impl PSStream {
    fn new(sid: u8, cid: u8) -> Self {
        Self {
            sid,
            cid,
            pts: 0,
            dts: 0,
            stream_buf: Vec::with_capacity(4096),
        }
    }
}

pub type OnFrameFn = dyn FnMut(&[u8], u8, u64, u64);
pub type OnPacketFn = dyn FnMut(&PSPacket, Result<(), PSError>);

pub struct PSDemuxer {
    stream_map: HashMap<u8, PSStream>,
    pkg: PSPacket,
    mpeg1: bool,
    cache: Vec<u8>,
    on_frame: Option<Box<OnFrameFn>>,
    on_packet: Option<Box<OnPacketFn>>,
}

impl PSDemuxer {
    pub fn new() -> Self {
        Self {
            stream_map: HashMap::new(),
            pkg: PSPacket::new(),
            mpeg1: false,
            cache: Vec::with_capacity(256),
            on_frame: None,
            on_packet: None,
        }
    }

    pub fn pkg(&self) -> &PSPacket {
        &self.pkg
    }

    // set_on_frame set_on_frame(Box::new(move |frame: &[u8], cid: u8, _pts: u64, _dts: u64| { }))
    pub fn set_on_frame(&mut self, cb: Box<OnFrameFn>) {
        self.on_frame = Some(cb);
    }

    pub fn set_on_packet(&mut self, cb: Box<OnPacketFn>) {
        self.on_packet = Some(cb);
    }


    pub fn input(&mut self, data: &[u8]) -> Result<(), PSError> {
        let mut bs_data: Vec<u8>;
        if !self.cache.is_empty() {
            bs_data = Vec::with_capacity(self.cache.len() + data.len());
            bs_data.extend_from_slice(&self.cache);
            bs_data.extend_from_slice(data);
        } else {
            bs_data = data.to_vec();
        }
        let mut bs = BitStream::new(&bs_data);

        let mut ret: Result<(), PSError> = Ok(());
        loop {
            if let Err(ref e) = ret {
                if e.need_more() {
                    let remain_bytes = bs.remain_bytes();
                    if remain_bytes > 0 {
                        let remain_data = bs.remain_data();
                        self.cache.clear();
                        self.cache.extend_from_slice(remain_data);
                    } else {
                        self.cache.clear();
                    }
                }
                break;
            }
            if bs.eos() {
                break;
            }
            if bs.remain_bits() < 32 {
                ret = Err(PSError::NeedMore);
                let remain_bytes = bs.remain_bytes();
                if remain_bytes > 0 {
                    let remain_data = bs.remain_data();
                    self.cache.clear();
                    self.cache.extend_from_slice(remain_data);
                } else {
                    self.cache.clear();
                }
                break;
            }
            let prefix_code = bs.next_bits(32) as u32;
            match prefix_code {
                0x000001BA => {
                    if self.pkg.header.is_none() {
                        self.pkg.header = Some(PSPackHeader::new());
                    }
                    if let Some(ref mut hdr) = self.pkg.header {
                        ret = hdr.decode(&mut bs);
                        self.mpeg1 = hdr.is_mpeg1;
                    }
                    if let Some(ref mut cb) = self.on_packet {
                        cb(&self.pkg, ret.clone());
                    }
                }
                0x000001BB => {
                    if self.pkg.header.is_none() {
                        ret = Err(PSError::InternalError(
                            "psdemuxer.pkg.Header must not be nil".to_string(),
                        ));
                        break;
                    }
                    if self.pkg.system.is_none() {
                        self.pkg.system = Some(SystemHeader::new());
                    }
                    if let Some(ref mut sys) = self.pkg.system {
                        ret = sys.decode(&mut bs);
                    }
                    if let Some(ref mut cb) = self.on_packet {
                        cb(&self.pkg, ret.clone());
                    }
                }
                0x000001BC => {
                    if self.pkg.psm.is_none() {
                        self.pkg.psm = Some(ProgramStreamMap::new());
                    }
                    if let Some(ref mut psm) = self.pkg.psm {
                        ret = psm.decode(&mut bs);
                        if ret.is_ok() {
                            for streaminfo in &psm.stream_map {
                                if !self
                                    .stream_map
                                    .contains_key(&streaminfo.elementary_stream_id)
                                {
                                    let stream = PSStream::new(
                                        streaminfo.elementary_stream_id,
                                        streaminfo.stream_type,
                                    );
                                    self.stream_map.insert(stream.sid, stream);
                                }
                            }
                        }
                    }
                    if let Some(ref mut cb) = self.on_packet {
                        cb(&self.pkg, ret.clone());
                    }
                }
                0x000001BD
                | 0x000001BE
                | 0x000001BF
                | 0x000001F0
                | 0x000001F1
                | 0x000001F2
                | 0x000001F3
                | 0x000001F4
                | 0x000001F5
                | 0x000001F6
                | 0x000001F7
                | 0x000001F8
                | 0x000001F9
                | 0x000001FA
                | 0x000001FB => {
                    if self.pkg.common_pes.is_none() {
                        self.pkg.common_pes = Some(CommonPesPacket::new());
                    }
                    if let Some(ref mut cp) = self.pkg.common_pes {
                        ret = cp.decode(&mut bs);
                    }
                }
                0x000001FF => {
                    if self.pkg.psd.is_none() {
                        self.pkg.psd = Some(ProgramStreamDirectory::new());
                    }
                    if let Some(ref mut psd) = self.pkg.psd {
                        ret = psd.decode(&mut bs);
                    }
                }
                0x000001B9 => {
                    continue;
                }
                _ => {
                    if (prefix_code & 0xFFFFFFE0) == 0x000001C0
                        || (prefix_code & 0xFFFFFFE0) == 0x000001E0
                    {
                        if self.pkg.pes.is_none() {
                            self.pkg.pes = Some(PesPacket::new());
                        }
                        if let Some(ref mut pes) = self.pkg.pes {
                            if self.mpeg1 {
                                ret = pes.decode_mpeg1(&mut bs);
                            } else {
                                ret = pes.decode(&mut bs);
                            }
                        }
                        if let Some(ref mut cb) = self.on_packet {
                            cb(&self.pkg, ret.clone());
                        }
                        if ret.is_ok() {
                            let stream_id = self
                                .pkg
                                .pes
                                .as_ref()
                                .map(|p| p.stream_id)
                                .unwrap_or(0);
                            let has_stream = self.stream_map.contains_key(&stream_id);
                            let is_mpeg1 = self.mpeg1;

                            if has_stream {
                                let mut stream =
                                    self.stream_map.remove(&stream_id).unwrap();
                                let pes = self.pkg.pes.as_ref().unwrap();
                                if is_mpeg1 && stream.cid == PS_STREAM_UNKNOW {
                                    Self::guess_codec_id(&mut stream);
                                }
                                Self::demux_pes_packet(&mut stream, pes, &mut self.on_frame);
                                self.stream_map.insert(stream_id, stream);
                            } else if is_mpeg1 {
                                let pes = self.pkg.pes.as_ref().unwrap();
                                let mut stream =
                                    PSStream::new(stream_id, PS_STREAM_UNKNOW);
                                stream.stream_buf.extend_from_slice(&pes.pes_payload);
                                stream.pts = pes.pts;
                                stream.dts = pes.dts;
                                self.stream_map.insert(stream_id, stream);
                            }
                        }
                    } else {
                        bs.skip_bits(40);
                    }
                }
            }
        }

        if ret.is_ok() && !self.cache.is_empty() {
            self.cache.clear();
        }

        ret
    }

    pub fn flush(&mut self) {
        let sids: Vec<u8> = self.stream_map.keys().cloned().collect();
        for sid in sids {
            if let Some(stream) = self.stream_map.get_mut(&sid) {
                if stream.stream_buf.is_empty() {
                    continue;
                }
                if let Some(ref mut cb) = self.on_frame {
                    cb(&stream.stream_buf, stream.cid, stream.pts / 90, stream.dts / 90);
                }
                stream.stream_buf.clear();
            }
        }
    }

    fn guess_codec_id(stream: &mut PSStream) {
        if (stream.sid & 0xE0) == PES_STREAM_AUDIO {
            stream.cid = PS_STREAM_AAC;
        } else if (stream.sid & 0xE0) == PES_STREAM_VIDEO {
            let mut h264score: i32 = 0;
            let mut h265score: i32 = 0;
            let data = stream.stream_buf.clone();
            split_frame(&data, |nalu| {
                let h264nalutype = h264_nalu_type_without_start_code(nalu);
                let h265nalutype = h265_nalu_type_without_start_code(nalu);
                if h264nalutype == H264NalType::PPS
                    || h264nalutype == H264NalType::SPS
                    || h264nalutype == H264NalType::ISlice
                {
                    h264score += 2;
                } else if (h264nalutype as u8) < 5 {
                    h264score += 1;
                } else if (h264nalutype as u8) > 20 {
                    h264score -= 1;
                }

                let h265_val = h265nalutype as u16;
                if h265nalutype == H265NalType::PPS
                    || h265nalutype == H265NalType::SPS
                    || h265nalutype == H265NalType::VPS
                    || (h265_val >= H265NalType::SliceBlaWlp as u16
                        && h265_val <= H265NalType::SliceCra as u16)
                {
                    h265score += 2;
                } else if h265_val >= H265NalType::SliceTrailN as u16
                    && h265_val <= H265NalType::SliceRaslR as u16
                {
                    h265score += 1;
                } else if h265_val > 40 {
                    h265score -= 1;
                }
                true
            });

            if h264score > h265score && h264score >= 4 {
                stream.cid = PS_STREAM_H264;
            } else if h264score < h265score && h265score >= 4 {
                stream.cid = PS_STREAM_H265;
            }
        }
    }

    fn demux_pes_packet(
        stream: &mut PSStream,
        pes: &PesPacket,
        on_frame: &mut Option<Box<OnFrameFn>>,
    ) {
        match stream.cid {
            PS_STREAM_AAC | PS_STREAM_G711A | PS_STREAM_G711U => {
                Self::demux_audio(stream, pes, on_frame);
            }
            PS_STREAM_H264 | PS_STREAM_H265 => {
                Self::demux_h26x(stream, pes, on_frame);
            }
            PS_STREAM_UNKNOW => {
                if stream.pts != pes.pts {
                    stream.stream_buf.clear();
                }
                stream.stream_buf.extend_from_slice(&pes.pes_payload);
                stream.pts = pes.pts;
                stream.dts = pes.dts;
            }
            _ => {}
        }
    }

    fn demux_audio(
        stream: &mut PSStream,
        pes: &PesPacket,
        on_frame: &mut Option<Box<OnFrameFn>>,
    ) {
        if stream.pts != pes.pts && !stream.stream_buf.is_empty() {
            if let Some(ref mut cb) = on_frame {
                cb(&stream.stream_buf, stream.cid, stream.pts / 90, stream.dts / 90);
            }
            stream.stream_buf.clear();
        }
        stream.stream_buf.extend_from_slice(&pes.pes_payload);
        stream.pts = pes.pts;
        stream.dts = pes.dts;
    }

    fn demux_h26x(
        stream: &mut PSStream,
        pes: &PesPacket,
        on_frame: &mut Option<Box<OnFrameFn>>,
    ) {
        if stream.stream_buf.is_empty() {
            stream.pts = pes.pts;
            stream.dts = pes.dts;
        }
        stream.stream_buf.extend_from_slice(&pes.pes_payload);

        let (mut start_i32, mut sc): (i32, i32) = find_start_code(&stream.stream_buf, 0);
        while start_i32 >= 0 {
            let search_from = (start_i32 as usize) + sc as usize;
            let (end_i32, sc2): (i32, i32) = find_start_code(&stream.stream_buf, search_from);
            if end_i32 < 0 {
                break;
            }
            let start = start_i32 as usize;
            let end = end_i32 as usize;
            let emit = if stream.cid == PS_STREAM_H264 {
                let nalu_type = h264_nalu_type(&stream.stream_buf[start..]);
                nalu_type != H264NalType::AUD
            } else if stream.cid == PS_STREAM_H265 {
                let nalu_type = h265_nalu_type(&stream.stream_buf[start..]);
                nalu_type != H265NalType::AUD
            } else {
                false
            };
            if emit {
                if let Some(ref mut cb) = on_frame {
                    cb(
                        &stream.stream_buf[start..end],
                        stream.cid,
                        stream.pts / 90,
                        stream.dts / 90,
                    );
                }
            }
            start_i32 = end_i32;
            sc = sc2;
        }
        if start_i32 >= 0 {
            stream.stream_buf = stream.stream_buf[start_i32 as usize..].to_vec();
        } else {
            stream.stream_buf.clear();
        }
        stream.pts = pes.pts;
        stream.dts = pes.dts;
    }
}

impl Default for PSDemuxer {
    fn default() -> Self {
        Self::new()
    }
}

impl Display for PSPackHeader {
    fn pretty_print(&self) {}
}

impl Display for SystemHeader {
    fn pretty_print(&self) {}
}

impl Display for ProgramStreamMap {
    fn pretty_print(&self) {}
}

impl Display for ProgramStreamDirectory {
    fn pretty_print(&self) {}
}

impl Display for CommonPesPacket {
    fn pretty_print(&self) {}
}

impl Display for PesPacket {
    fn pretty_print(&self) {}
}