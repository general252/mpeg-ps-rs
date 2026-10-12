use crate::codec::H265NalType;

pub const H265_NAL_AUD: i32 = 35;
pub const H265_NAL_SLICE_BLA_W_LP: i32 = 16;
pub const H265_NAL_SLICE_CRA: i32 = 21;

pub fn h265_nalu_type_without_start_code(nalu: &[u8]) -> H265NalType {
    H265NalType::from_i32(((nalu[0] as i32) >> 1) & 0x3F)
}

pub fn is_h265_vcl_nalu_type(nal_type: H265NalType) -> bool {
    let t = nal_type as i32;
    (t >= H265NalType::SliceBlaWlp as i32 && t <= H265NalType::SliceCra as i32)
        || (t >= H265NalType::SliceTrailN as i32 && t <= H265NalType::SliceRaslR as i32)
}
