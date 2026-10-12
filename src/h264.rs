use crate::codec::H264NalType;

pub const H264_NAL_AUD: i32 = 9;

pub fn h264_nalu_type_without_start_code(nalu: &[u8]) -> H264NalType {
    H264NalType::from_i32(nalu[0] as i32 & 0x1F)
}

pub fn is_h264_vcl_nalu_type(nal_type: H264NalType) -> bool {
    let t = nal_type as i32;
    t > H264NalType::Reserved as i32 && t <= H264NalType::ISlice as i32
}
