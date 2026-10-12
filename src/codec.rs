#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CodecID {
    VideoH264 = 1,
    VideoH265 = 2,
    VideoVP8 = 3,
    AudioAAC = 98,
    AudioG711A = 99,
    AudioG711U = 100,
    AudioOpus = 101,
    AudioMP3 = 102,
    Unrecognized = 999,
}

impl CodecID {
    pub fn from_i32(v: i32) -> Self {
        match v {
            1 => CodecID::VideoH264,
            2 => CodecID::VideoH265,
            3 => CodecID::VideoVP8,
            98 => CodecID::AudioAAC,
            99 => CodecID::AudioG711A,
            100 => CodecID::AudioG711U,
            101 => CodecID::AudioOpus,
            102 => CodecID::AudioMP3,
            _ => CodecID::Unrecognized,
        }
    }

    pub fn to_str(&self) -> &str {
        match self {
            CodecID::VideoH264 => "H264",
            CodecID::VideoH265 => "H265",
            CodecID::VideoVP8 => "VP8",
            CodecID::AudioAAC => "AAC",
            CodecID::AudioG711A => "G711A",
            CodecID::AudioG711U => "G711U",
            CodecID::AudioOpus => "OPUS",
            CodecID::AudioMP3 => "MP3",
            CodecID::Unrecognized => "UNRECOGNIZED",
        }
    }
}

#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum H264NalType {
    Reserved = 0,
    PSlice,
    SliceA,
    SliceB,
    SliceC,
    ISlice,
    SEI,
    SPS,
    PPS,
    AUD,
}

impl H264NalType {
    pub fn from_i32(v: i32) -> Self {
        match v {
            0 => H264NalType::Reserved,
            1 => H264NalType::PSlice,
            2 => H264NalType::SliceA,
            3 => H264NalType::SliceB,
            4 => H264NalType::SliceC,
            5 => H264NalType::ISlice,
            6 => H264NalType::SEI,
            7 => H264NalType::SPS,
            8 => H264NalType::PPS,
            9 => H264NalType::AUD,
            _ => H264NalType::Reserved,
        }
    }
}

#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum H265NalType {
    SliceTrailN = 0,
    LiceTrailR,
    SliceTsaN,
    SliceTsaR,
    SliceStsaN,
    SliceStsaR,
    SliceRadlN,
    SliceRadlR,
    SliceRaslN,
    SliceRaslR,
    SliceBlaWlp = 16,
    SliceBlaWradl,
    SliceBlaNlp,
    SliceIdrWradl,
    SliceIdrNlp,
    SliceCra,
    VPS = 32,
    SPS,
    PPS,
    AUD,
    SEI,
    SEISuffix,
}

impl H265NalType {
    pub fn from_i32(v: i32) -> Self {
        match v {
            0 => H265NalType::SliceTrailN,
            1 => H265NalType::LiceTrailR,
            2 => H265NalType::SliceTsaN,
            3 => H265NalType::SliceTsaR,
            4 => H265NalType::SliceStsaN,
            5 => H265NalType::SliceStsaR,
            6 => H265NalType::SliceRadlN,
            7 => H265NalType::SliceRadlR,
            8 => H265NalType::SliceRaslN,
            9 => H265NalType::SliceRaslR,
            16 => H265NalType::SliceBlaWlp,
            17 => H265NalType::SliceBlaWradl,
            18 => H265NalType::SliceBlaNlp,
            19 => H265NalType::SliceIdrWradl,
            20 => H265NalType::SliceIdrNlp,
            21 => H265NalType::SliceCra,
            32 => H265NalType::VPS,
            33 => H265NalType::SPS,
            34 => H265NalType::PPS,
            35 => H265NalType::AUD,
            39 => H265NalType::SEI,
            40 => H265NalType::SEISuffix,
            _ => H265NalType::SliceTrailN,
        }
    }
}
