use crate::bitstream::BitStreamWriter;
use crate::codec::{H264NalType, H265NalType};
use crate::error::PSError;
use crate::h264::{h264_nalu_type_without_start_code, is_h264_vcl_nalu_type};
use crate::h265::{h265_nalu_type_without_start_code, is_h265_vcl_nalu_type};
use crate::pes_proto::{H264_AUD_NALU, H265_AUD_NALU, PesPacket};
use crate::ps_proto::{
    ElementaryStream, ElementaryStreamElem, PES_STREAM_AUDIO, PES_STREAM_VIDEO,
    PS_STREAM_H264, PS_STREAM_H265, PSPackHeader, ProgramStreamMap, SystemHeader,
};
use crate::util::split_frame;

pub struct PSMuxer {
    system: SystemHeader,
    psm: ProgramStreamMap,
    on_packet: Option<Box<dyn FnMut(&[u8])>>,
    firstframe: bool,
    packet_buf: Vec<Vec<u8>>,
}

impl PSMuxer {
    pub fn new() -> Self {
        let mut muxer = Self {
            system: SystemHeader::new(),
            psm: ProgramStreamMap::new(),
            on_packet: None,
            firstframe: true,
            packet_buf: Vec::new(),
        };
        muxer.system.rate_bound = 26234;
        muxer.psm.current_next_indicator = 1;
        muxer.psm.program_stream_map_version = 1;
        muxer
    }

    pub fn packet_data(&mut self) -> Vec<u8> {
        let mut result = Vec::new();
        for pkt in &self.packet_buf {
            result.extend_from_slice(pkt);
        }
        self.packet_buf.clear();
        result
    }

    pub fn set_on_packet<F>(&mut self, callback: F)
    where
        F: FnMut(&[u8]) + 'static,
    {
        self.on_packet = Some(Box::new(callback));
    }

    pub fn system(&self) -> &SystemHeader {
        &self.system
    }

    pub fn system_mut(&mut self) -> &mut SystemHeader {
        &mut self.system
    }

    pub fn psm(&self) -> &ProgramStreamMap {
        &self.psm
    }

    pub fn psm_mut(&mut self) -> &mut ProgramStreamMap {
        &mut self.psm
    }

    pub fn add_stream(&mut self, cid: u8) -> u8 {
        if cid == PS_STREAM_H265 || cid == PS_STREAM_H264 {
            let mut es = ElementaryStream::new(PES_STREAM_VIDEO + self.system.video_bound);
            es.p_std_buffer_bound_scale = 1;
            es.p_std_buffer_size_bound = 400;
            let sid = es.stream_id;
            self.system.streams.push(es);
            self.system.video_bound += 1;
            self.psm
                .stream_map
                .push(ElementaryStreamElem::new(cid, sid));
            self.psm.program_stream_map_version += 1;
            sid
        } else {
            let mut es = ElementaryStream::new(PES_STREAM_AUDIO + self.system.audio_bound);
            es.p_std_buffer_bound_scale = 0;
            es.p_std_buffer_size_bound = 32;
            let sid = es.stream_id;
            self.system.streams.push(es);
            self.system.audio_bound += 1;
            self.psm
                .stream_map
                .push(ElementaryStreamElem::new(cid, sid));
            self.psm.program_stream_map_version += 1;
            sid
        }
    }

    pub fn write(&mut self, sid: u8, frame: &[u8], pts: u64, dts: u64) -> Result<(), PSError> {
        let stream = self
            .psm
            .stream_map
            .iter()
            .find(|es| es.elementary_stream_id == sid);

        if stream.is_none() {
            return Err(PSError::StreamIdNotFound);
        }
        let stream = stream.unwrap();

        if frame.is_empty() {
            return Ok(());
        }

        let mut withaud: bool = false;
        let mut idr_flag: bool = false;
        let mut vcl: bool = false;

        if stream.stream_type == PS_STREAM_H264 || stream.stream_type == PS_STREAM_H265 {
            let stream_type = stream.stream_type;
            let withaud_ref = &mut withaud;
            let idr_flag_ref = &mut idr_flag;
            let vcl_ref = &mut vcl;

            split_frame(frame, move |nalu| {
                if stream_type == PS_STREAM_H264 {
                    let nalu_type = h264_nalu_type_without_start_code(nalu);
                    if nalu_type == H264NalType::AUD {
                        *withaud_ref = true;
                        return false;
                    } else if is_h264_vcl_nalu_type(nalu_type) {
                        if nalu_type == H264NalType::ISlice {
                            *idr_flag_ref = true;
                        }
                        *vcl_ref = true;
                        return false;
                    }
                    true
                } else {
                    let nalu_type = h265_nalu_type_without_start_code(nalu);
                    let nt = nalu_type as i32;
                    if nalu_type == H265NalType::AUD {
                        *withaud_ref = true;
                        return false;
                    } else if is_h265_vcl_nalu_type(nalu_type) {
                        if nt >= H265NalType::SliceBlaWlp as i32
                            && nt <= H265NalType::SliceCra as i32
                        {
                            *idr_flag_ref = true;
                        }
                        *vcl_ref = true;
                        return false;
                    }
                    true
                }
            });
        }

        let dts = dts * 90;
        let pts = pts * 90;
        let mut bsw = BitStreamWriter::new(1024);

        let mut pack = PSPackHeader::new();
        pack.system_clock_reference_base = dts.wrapping_sub(3600);
        pack.system_clock_reference_extension = 0;
        pack.program_mux_rate = 6106;
        pack.encode(&mut bsw);

        if self.firstframe || idr_flag {
            self.system.encode(&mut bsw);
            self.psm.encode(&mut bsw);
            self.firstframe = false;
        }

        let stream_type = stream.stream_type;
        let mut pespkg = PesPacket::new();
        let mut first = true;
        let mut remaining = frame;

        while !remaining.is_empty() {
            let mut peshdrlen: usize = 13;
            pespkg.stream_id = sid;
            pespkg.pts_dts_flags = 0x03;
            pespkg.pes_header_data_length = 10;
            pespkg.pts = pts;
            pespkg.dts = dts;
            pespkg.data_alignment_indicator = if idr_flag { 1 } else { 0 };

            if first && !withaud && vcl {
                if stream_type == PS_STREAM_H264 {
                    pespkg
                        .pes_payload
                        .extend_from_slice(&H264_AUD_NALU);
                    peshdrlen += 6;
                } else if stream_type == PS_STREAM_H265 {
                    pespkg
                        .pes_payload
                        .extend_from_slice(&H265_AUD_NALU);
                    peshdrlen += 7;
                }
            }

            if peshdrlen + remaining.len() >= 0xFFFF {
                pespkg.pes_packet_length = 0xFFFF;
                let take = 0xFFFF - peshdrlen;
                pespkg
                    .pes_payload
                    .extend_from_slice(&remaining[..take]);
                remaining = &remaining[take..];
            } else {
                pespkg.pes_packet_length = (peshdrlen + remaining.len()) as u16;
                pespkg.pes_payload.extend_from_slice(remaining);
                remaining = &[];
            }

            pespkg.encode(&mut bsw);

            let bits = bsw.bits().to_vec();
            self.packet_buf.push(bits);

            if let Some(ref mut cb) = self.on_packet {
                cb(bsw.bits());
            }

            bsw.reset();
            pespkg.clear_payload();
            first = false;
        }

        Ok(())
    }
}

impl Default for PSMuxer {
    fn default() -> Self {
        Self::new()
    }
}
