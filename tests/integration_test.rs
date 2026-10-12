use mpeg_ps::*;
use std::sync::{Arc, Mutex};

#[test]
fn test_ps_muxer_creation() {
    let muxer = PSMuxer::new();
    assert_eq!(muxer.system().rate_bound, 26234);
    assert_eq!(muxer.psm().current_next_indicator, 1);
    assert_eq!(muxer.psm().program_stream_map_version, 1);
}

#[test]
fn test_add_h264_stream() {
    let mut muxer = PSMuxer::new();
    let sid = muxer.add_stream(PS_STREAM_H264);

    assert_eq!(sid, PES_STREAM_VIDEO);
    assert_eq!(muxer.system().video_bound, 1);
    assert_eq!(muxer.system().streams.len(), 1);
    assert_eq!(muxer.system().streams[0].stream_id, PES_STREAM_VIDEO);
    assert_eq!(muxer.system().streams[0].p_std_buffer_bound_scale, 1);
    assert_eq!(muxer.system().streams[0].p_std_buffer_size_bound, 400);
    assert_eq!(muxer.psm().stream_map.len(), 1);
    assert_eq!(muxer.psm().stream_map[0].stream_type, PS_STREAM_H264);
    assert_eq!(muxer.psm().stream_map[0].elementary_stream_id, sid);
    assert_eq!(muxer.psm().program_stream_map_version, 2);
}

#[test]
fn test_add_h265_stream() {
    let mut muxer = PSMuxer::new();
    let sid = muxer.add_stream(PS_STREAM_H265);

    assert_eq!(sid, PES_STREAM_VIDEO);
    assert_eq!(muxer.system().video_bound, 1);
    assert_eq!(muxer.psm().stream_map[0].stream_type, PS_STREAM_H265);
}

#[test]
fn test_add_aac_stream() {
    let mut muxer = PSMuxer::new();
    let sid = muxer.add_stream(PS_STREAM_AAC);

    assert_eq!(sid, PES_STREAM_AUDIO);
    assert_eq!(muxer.system().audio_bound, 1);
    assert_eq!(muxer.system().streams[0].stream_id, PES_STREAM_AUDIO);
    assert_eq!(muxer.system().streams[0].p_std_buffer_bound_scale, 0);
    assert_eq!(muxer.system().streams[0].p_std_buffer_size_bound, 32);
    assert_eq!(muxer.psm().stream_map[0].stream_type, PS_STREAM_AAC);
}

#[test]
fn test_add_multiple_streams() {
    let mut muxer = PSMuxer::new();
    let v1 = muxer.add_stream(PS_STREAM_H264);
    let v2 = muxer.add_stream(PS_STREAM_H265);
    let a1 = muxer.add_stream(PS_STREAM_AAC);

    assert_eq!(v1, PES_STREAM_VIDEO);
    assert_eq!(v2, PES_STREAM_VIDEO + 1);
    assert_eq!(a1, PES_STREAM_AUDIO);
    assert_eq!(muxer.system().video_bound, 2);
    assert_eq!(muxer.system().audio_bound, 1);
    assert_eq!(muxer.system().streams.len(), 3);
    assert_eq!(muxer.psm().stream_map.len(), 3);
}

#[test]
fn test_write_not_found_stream() {
    let mut muxer = PSMuxer::new();
    let result = muxer.write(0x99, &[0x00; 10], 0, 0);
    assert!(result.is_err());
    assert!(result.unwrap_err().stream_id_not_found());
}

#[test]
fn test_write_empty_frame() {
    let mut muxer = PSMuxer::new();
    let sid = muxer.add_stream(PS_STREAM_H264);
    let result = muxer.write(sid, &[], 0, 0);
    assert!(result.is_ok());
}

#[test]
fn test_write_h264_frame() {
    let mut muxer = PSMuxer::new();
    let sid = muxer.add_stream(PS_STREAM_H264);

    let packets = Arc::new(Mutex::new(Vec::new()));
    let pkts = packets.clone();
    muxer.set_on_packet(move |pkg| {
        pkts.lock().unwrap().push(pkg.to_vec());
    });

    let mut frame = vec![0x00, 0x00, 0x00, 0x01, 0x41, 0x9A, 0x21, 0x6C];
    frame.extend_from_slice(&[0x00; 200]);

    muxer.write(sid, &frame, 1000, 1000).unwrap();

    let pkts = packets.lock().unwrap();
    assert!(!pkts.is_empty());

    let first_pkt = &pkts[0];
    assert!(first_pkt.len() > 14);
    assert_eq!(&first_pkt[0..4], &[0x00, 0x00, 0x01, 0xBA]);
}

#[test]
fn test_write_aac_frame() {
    let mut muxer = PSMuxer::new();
    let sid = muxer.add_stream(PS_STREAM_AAC);

    let packets = Arc::new(Mutex::new(Vec::new()));
    let pkts = packets.clone();
    muxer.set_on_packet(move |pkg| {
        pkts.lock().unwrap().push(pkg.to_vec());
    });

    let mut frame = vec![0xFF, 0xF1, 0x50, 0x80, 0x00, 0x00];
    frame.extend_from_slice(&[0x00; 100]);

    muxer.write(sid, &frame, 2000, 2000).unwrap();

    let pkts = packets.lock().unwrap();
    assert!(!pkts.is_empty());

    let first_pkt = &pkts[0];
    assert_eq!(&first_pkt[0..4], &[0x00, 0x00, 0x01, 0xBA]);
}

#[test]
fn test_multiple_write_calls() {
    let mut muxer = PSMuxer::new();
    let vsid = muxer.add_stream(PS_STREAM_H264);
    let asid = muxer.add_stream(PS_STREAM_AAC);

    let packets = Arc::new(Mutex::new(Vec::new()));
    let pkts = packets.clone();
    muxer.set_on_packet(move |pkg| {
        pkts.lock().unwrap().push(pkg.to_vec());
    });

    let mut h264_frame = vec![0x00, 0x00, 0x00, 0x01, 0x41, 0x9A, 0x21, 0x6C];
    h264_frame.extend_from_slice(&[0x00; 100]);

    let mut aac_frame = vec![0xFF, 0xF1, 0x50, 0x80, 0x00, 0x00];
    aac_frame.extend_from_slice(&[0x00; 50]);

    muxer.write(vsid, &h264_frame, 0, 0).unwrap();
    muxer.write(asid, &aac_frame, 33, 33).unwrap();

    let pkts = packets.lock().unwrap();
    assert!(pkts.len() >= 2);
}

#[test]
fn test_ps_pack_header_encode() {
    let mut bsw = BitStreamWriter::new(256);
    let mut pack = PSPackHeader::new();
    pack.system_clock_reference_base = 36000;
    pack.system_clock_reference_extension = 0;
    pack.program_mux_rate = 6106;

    pack.encode(&mut bsw);
    let data = bsw.bits();

    assert_eq!(&data[0..4], &[0x00, 0x00, 0x01, 0xBA]);
    assert!(data.len() >= 14);
}

#[test]
fn test_system_header_encode() {
    let mut bsw = BitStreamWriter::new(256);
    let mut sh = SystemHeader::new();
    sh.rate_bound = 26234;
    sh.video_bound = 1;
    sh.audio_bound = 1;

    let mut es = ElementaryStream::new(PES_STREAM_VIDEO);
    es.p_std_buffer_bound_scale = 1;
    es.p_std_buffer_size_bound = 400;
    sh.streams.push(es);

    sh.encode(&mut bsw);
    let data = bsw.bits();

    assert_eq!(&data[0..4], &[0x00, 0x00, 0x01, 0xBB]);
    assert!(data.len() >= 6);
}

#[test]
fn test_program_stream_map_encode() {
    let mut bsw = BitStreamWriter::new(256);
    let mut psm = ProgramStreamMap::new();
    psm.current_next_indicator = 1;
    psm.program_stream_map_version = 2;

    psm.stream_map
        .push(ElementaryStreamElem::new(PS_STREAM_H264, PES_STREAM_VIDEO));

    psm.encode(&mut bsw);
    let data = bsw.bits();

    assert_eq!(&data[0..4], &[0x00, 0x00, 0x01, 0xBC]);
    assert!(data.len() >= 10);
}

#[test]
fn test_pes_packet_encode() {
    let mut bsw = BitStreamWriter::new(256);
    let mut pes = PesPacket::new();
    pes.stream_id = PES_STREAM_VIDEO;
    pes.pts_dts_flags = 0x03;
    pes.pes_header_data_length = 10;
    pes.pts = 90000;
    pes.dts = 90000;
    pes.pes_packet_length = (13 + 10) as u16;
    pes.pes_payload = vec![0x00; 10];

    pes.encode(&mut bsw);
    let data = bsw.bits();

    assert_eq!(&data[0..3], &[0x00, 0x00, 0x01]);
    assert_eq!(data[3], PES_STREAM_VIDEO);
}

#[test]
fn test_calc_crc32() {
    let data: Vec<u8> = (0u8..=255).collect();
    let crc = calc_crc32(0xFFFFFFFF, &data);
    assert_eq!(crc, 0x6A114A49);
}

#[test]
fn test_find_start_code() {
    let data = [0x00, 0x00, 0x01, 0x01, 0x00, 0x00, 0x00, 0x01, 0x02];
    let (pos, sc) = find_start_code(&data, 0);
    assert_eq!(pos, 0);
    assert_eq!(sc, 3);

    let (pos2, sc2) = find_start_code(&data, pos as usize + sc as usize);
    assert_eq!(pos2, 4);
    assert_eq!(sc2, 4);
}

#[test]
fn test_split_frame() {
    let data = vec![
        0x00, 0x00, 0x00, 0x01, 0x41, 0x00, 0x00, 0x00, 0x00, 0x01, 0x42, 0x00, 0x00,
    ];
    let mut nalu_count = 0;
    split_frame(&data, |_nalu| {
        nalu_count += 1;
        true
    });
    assert_eq!(nalu_count, 2);
}

#[test]
fn test_split_frame_early_stop() {
    let data = vec![
        0x00, 0x00, 0x00, 0x01, 0x41, 0x00, 0x00, 0x00, 0x00, 0x01, 0x42, 0x00, 0x00,
    ];
    let mut nalu_count = 0;
    split_frame(&data, |_nalu| {
        nalu_count += 1;
        if nalu_count == 1 {
            false
        } else {
            true
        }
    });
    assert_eq!(nalu_count, 1);
}

#[test]
fn test_h264_nalu_type_detection() {
    let nalu = [0x41, 0x9A];
    let ty = h264_nalu_type_without_start_code(&nalu);
    assert_eq!(ty, H264NalType::PSlice);

    let aud = [0x09, 0xF0];
    let aud_type = h264_nalu_type_without_start_code(&aud);
    assert_eq!(aud_type, H264NalType::AUD);
}

#[test]
fn test_h265_nalu_type_detection() {
    let nalu = [0x46, 0x01];
    let ty = h265_nalu_type_without_start_code(&nalu);
    assert_eq!(ty, H265NalType::AUD);

    let slice = [0x00, 0x01];
    let slice_type = h265_nalu_type_without_start_code(&slice);
    assert_eq!(slice_type, H265NalType::SliceTrailN);
}

#[test]
fn test_bitstream_reader() {
    let data = [0xAB, 0xCD, 0xEF, 0x01];
    let mut bs = BitStream::new(&data);

    assert_eq!(bs.uint8(8), 0xAB);
    assert_eq!(bs.uint8(8), 0xCD);
    assert_eq!(bs.uint16(16), 0xEF01);
}

#[test]
fn test_bitstream_writer() {
    let mut bsw = BitStreamWriter::new(64);

    bsw.put_byte(0xAB);
    bsw.put_byte(0xCD);
    bsw.put_uint16(0xEF01, 16);

    assert_eq!(bsw.bits(), &[0xAB, 0xCD, 0xEF, 0x01]);
}

#[test]
fn test_bitstream_read_ue() {
    let data = [0x80];
    let mut bs = BitStream::new(&data);
    let val = bs.read_ue();
    assert_eq!(val, 0);
}

#[test]
fn test_error_types() {
    let err = PSError::NeedMore;
    assert!(err.need_more());
    assert!(!err.parser_error());
    assert!(!err.stream_id_not_found());

    let err2 = PSError::ParserError;
    assert!(err2.parser_error());

    let err3 = PSError::StreamIdNotFound;
    assert!(err3.stream_id_not_found());
}

#[test]
fn test_ps_demuxer_creation() {
    let _demuxer = PSDemuxer::new();
}

#[test]
fn test_ps_demuxer_input_pack_header() {
    let data = [
        0x00, 0x00, 0x01, 0xBA, 0x40, 0x01, 0x00, 0x01, 0x33, 0x44, 0xFF, 0xFF, 0xFF, 0xF1, 0xFF,
    ];
    let mut demuxer = PSDemuxer::new();
    let result = demuxer.input(&data);
    assert!(result.is_ok());
}

#[test]
fn test_ps_demuxer_input_need_more() {
    let data = [0x00, 0x00, 0x01, 0xBA];
    let mut demuxer = PSDemuxer::new();
    let result = demuxer.input(&data);
    assert!(result.is_err());
    assert!(result.unwrap_err().need_more());
}

#[test]
fn test_ps_demuxer_input_pack_and_system() {
    let data = [
        0x00, 0x00, 0x01, 0xba, 0x7f, 0xff, 0xff, 0x8f, 0x84, 0x01, 0x00, 0x5f, 0x6b, 0xf8, 0x00,
        0x00, 0x01, 0xbb, 0x00, 0x0c, 0x80, 0xcc, 0xf5, 0x04, 0x21, 0x7f, 0xe0, 0xe1, 0x90, 0xc0,
        0xc0, 0x20, 0x00, 0x00, 0x01, 0xbc, 0x00, 0x12, 0xe3, 0xff, 0x00, 0x00, 0x00, 0x08, 0x1b,
        0xe0, 0x00, 0x00, 0x0f, 0xc0, 0x00, 0x00, 0x5d, 0xe7, 0x77, 0x81, 0x00, 0x00, 0x01, 0xe0,
        0x00, 0x7f, 0x80, 0xc0, 0x0a, 0x31, 0x00, 0x01, 0x00, 0x01, 0x11, 0x00, 0x01, 0x00, 0x01,
        0x00, 0x00, 0x00, 0x01, 0x09, 0xf0, 0x00, 0x00, 0x00, 0x01, 0x41, 0x9a, 0x21, 0x6c, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0xba, 0x7f, 0xff,
        0xff, 0x8f, 0x84, 01, 0x00, 0x5f, 0x6b, 0xf8, 0x00, 0x00, 0x01, 0xc0, 0x00, 0x45, 0x80,
        0xc0, 0x0a, 0x31, 0x00, 01, 0x00, 0x01, 0x11, 0x00, 0x01, 0x00, 0x01, 0xff, 0xf1, 0x50,
        0x80, 0x00, 0x00, 0x00, 0x00, 00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    ];
    let mut demuxer = PSDemuxer::new();
    demuxer.set_on_frame(Box::new(|_frame: &[u8], _cid: u8, _pts: u64, _dts: u64| {
        println!(
            "on_frame {:?} {:?} {:?} {:?}",
            _frame.len(),
            _cid,
            _pts,
            _dts
        );
    }));

    let result = demuxer.input(&data);
    demuxer.flush();

    println!("{:?}", result);
    assert!(result.is_ok());
}

#[test]
fn test_ps_demuxer_input_psm() {
    let data = [
        0x00, 0x00, 0x01, 0xBC, 0x40, 0x0a, 0x00, 0x00, 0x00, 0x00, 0x00, 0x03, 0x34, 0x81, 0x00,
        0x00,
    ];
    let mut demuxer = PSDemuxer::new();
    let result = demuxer.input(&data);
    println!("{:?}", result);
    assert!(result.is_ok());
}

#[test]
fn test_ps_demuxer_input_mpeg1() {
    let data = [
        0x00, 0x00, 0x01, 0xBA, 0x20, 0x0a, 0x00, 0x00, 0x00, 0x00, 0x00, 0x03,
    ];
    let mut demuxer = PSDemuxer::new();
    let result = demuxer.input(&data);
    assert!(result.is_ok());
}

#[test]
fn test_ps_demuxer_pack_header_decode() {
    let data = [
        0x00, 0x00, 0x01, 0xBA, 0x40, 0x01, 0x00, 0x01, 0x33, 0x44, 0xFF, 0xFF, 0xFF, 0xF1, 0xFF,
    ];
    let mut demuxer = PSDemuxer::new();
    demuxer.input(&data).unwrap();
    let header = demuxer.pkg().header().unwrap();
    assert!(!header.is_mpeg1);
    assert_eq!(header.system_clock_reference_base, 0x400000001);
}

#[test]
fn test_ps_demuxer_system_header_decode() {
    let data = [
        0x00, 0x00, 0x01, 0xBA, 0x40, 0x01, 0x00, 0x01, 0x33, 0x44, 0xFF, 0xFF, 0xFF, 0xF1, 0x34,
        0x00, 0x00, 0x01, 0xBB, 0x00, 0x01, 0x00, 0x01, 0x33, 0x44, 0xFF, 0x34,
    ];
    let mut demuxer = PSDemuxer::new();
    demuxer.input(&data).unwrap();
    let sys = demuxer.pkg().system().unwrap();
    assert_eq!(sys.rate_bound, 0x040004);
}

#[test]
fn test_ps_demuxer_roundtrip_h264() {
    let mut muxer = PSMuxer::new();
    let sid = muxer.add_stream(PS_STREAM_H264);
    let frame = [0x00, 0x00, 0x00, 0x01, 0x65, 0x88, 0x84, 0x00];

    let packets = Arc::new(Mutex::new(Vec::new()));
    let pkts = packets.clone();

    let mut demuxer = PSDemuxer::new();
    demuxer.set_on_frame(Box::new(
        move |_frame: &[u8], _cid: u8, _pts: u64, _dts: u64| {
            pkts.lock().unwrap().push(_frame.to_vec());
        },
    ));

    muxer.write(sid, &frame, 90000, 90000).unwrap();
    let ps_data = muxer.packet_data();

    demuxer.input(&ps_data).unwrap();
    demuxer.flush();

    let pkts = packets.lock().unwrap();
    assert!(!pkts.is_empty());
    assert_eq!(pkts[0], frame);
}

#[test]
fn test_ps_demuxer_roundtrip_aac() {
    let mut muxer = PSMuxer::new();
    let sid = muxer.add_stream(PS_STREAM_AAC);
    let frame = [0xFF, 0xF1, 0x00, 0x50, 0x00, 0x00, 0x00, 0x00];

    let packets = Arc::new(Mutex::new(Vec::new()));
    let cids = Arc::new(Mutex::new(Vec::new()));
    let pts_list = Arc::new(Mutex::new(Vec::new()));

    let pkts = packets.clone();
    let cids_c = cids.clone();
    let pts_c = pts_list.clone();

    let mut demuxer = PSDemuxer::new();
    demuxer.set_on_frame(Box::new(
        move |_frame: &[u8], _cid: u8, _pts: u64, _dts: u64| {
            pkts.lock().unwrap().push(_frame.to_vec());
            cids_c.lock().unwrap().push(_cid);
            pts_c.lock().unwrap().push(_pts);
        },
    ));

    muxer.write(sid, &frame, 90000, 90000).unwrap();
    let ps_data = muxer.packet_data();

    demuxer.input(&ps_data).unwrap();
    demuxer.flush();

    let pkts = packets.lock().unwrap();
    let cids = cids.lock().unwrap();
    assert!(!pkts.is_empty());
    assert_eq!(cids[0], PS_STREAM_AAC);
    assert_eq!(pkts[0], frame);
}

#[test]
fn test_ps_demuxer_multiple_frames() {
    let mut muxer = PSMuxer::new();
    let sid = muxer.add_stream(PS_STREAM_H264);

    let frame1 = [0x00, 0x00, 0x00, 0x01, 0x65, 0x88, 0x84, 0x00];
    let frame2 = [0x00, 0x00, 0x00, 0x01, 0x41, 0x9A, 0x21, 0x60];

    let all_frames = Arc::new(Mutex::new(Vec::new()));
    let af = all_frames.clone();

    let mut demuxer = PSDemuxer::new();
    demuxer.set_on_frame(Box::new(
        move |_frame: &[u8], _cid: u8, _pts: u64, _dts: u64| {
            af.lock().unwrap().push(_frame.to_vec());
        },
    ));

    muxer.write(sid, &frame1, 90000, 90000).unwrap();
    muxer.write(sid, &frame2, 90030, 90030).unwrap();
    let ps_data = muxer.packet_data();

    demuxer.input(&ps_data).unwrap();
    demuxer.flush();

    let all_frames = all_frames.lock().unwrap();
    assert_eq!(all_frames.len(), 2);
}

#[test]
fn test_ps_demuxer_on_packet_callback() {
    let pack_count = Arc::new(Mutex::new(0));
    let system_count = Arc::new(Mutex::new(0));
    let psm_count = Arc::new(Mutex::new(0));

    let pc = pack_count.clone();
    let sc = system_count.clone();
    let pmc = psm_count.clone();

    let mut demuxer = PSDemuxer::new();
    demuxer.set_on_packet(Box::new(
        move |pkg: &PSPacket, result: Result<(), PSError>| {
            if result.is_ok() {
                if pkg.header.is_some() {
                    *pc.lock().unwrap() += 1;
                }
                if pkg.system.is_some() {
                    *sc.lock().unwrap() += 1;
                }
                if pkg.psm.is_some() {
                    *pmc.lock().unwrap() += 1;
                }
            }
        },
    ));

    let mut muxer = PSMuxer::new();
    let _sid = muxer.add_stream(PS_STREAM_H264);
    let ps_data = muxer.packet_data();

    demuxer.input(&ps_data).unwrap();
    demuxer.flush();

    assert!(*pack_count.lock().unwrap() > 0);
    assert!(*system_count.lock().unwrap() > 0);
    assert!(*psm_count.lock().unwrap() > 0);
}

#[test]
fn test_ps_demuxer_flush_empty() {
    let called = Arc::new(Mutex::new(false));
    let called_clone = called.clone();
    let mut demuxer = PSDemuxer::new();
    demuxer.set_on_frame(Box::new(
        move |_frame: &[u8], _cid: u8, _pts: u64, _dts: u64| {
            *called_clone.lock().unwrap() = true;
        },
    ));
    demuxer.flush();
    assert!(!*called.lock().unwrap());
}

#[test]
fn test_ps_demuxer_stream_map_creation() {
    let mut demuxer = PSDemuxer::new();
    let mut muxer = PSMuxer::new();
    muxer.add_stream(PS_STREAM_H264);
    muxer.add_stream(PS_STREAM_AAC);
    let ps_data = muxer.packet_data();

    demuxer.input(&ps_data).unwrap();
}

#[test]
fn test_ps_demuxer_multi_stream_roundtrip() {
    let mut muxer = PSMuxer::new();
    let video_sid = muxer.add_stream(PS_STREAM_H264);
    let audio_sid = muxer.add_stream(PS_STREAM_AAC);

    let video_frame = [0x00, 0x00, 0x00, 0x01, 0x65, 0x88, 0x84, 0x00];
    let audio_frame = [0xFF, 0xF1, 0x00, 0x50, 0x00, 0x00];

    let video_frames = Arc::new(Mutex::new(Vec::new()));
    let audio_frames = Arc::new(Mutex::new(Vec::new()));

    let vf = video_frames.clone();
    let af = audio_frames.clone();

    let mut demuxer = PSDemuxer::new();
    demuxer.set_on_frame(Box::new(
        move |_frame: &[u8], _cid: u8, _pts: u64, _dts: u64| {
            if _cid == PS_STREAM_H264 {
                vf.lock().unwrap().push(_frame.to_vec());
            } else if _cid == PS_STREAM_AAC {
                af.lock().unwrap().push(_frame.to_vec());
            }
        },
    ));

    muxer.write(video_sid, &video_frame, 90000, 90000).unwrap();
    muxer.write(audio_sid, &audio_frame, 90000, 90000).unwrap();
    let ps_data = muxer.packet_data();

    demuxer.input(&ps_data).unwrap();
    demuxer.flush();

    assert_eq!(video_frames.lock().unwrap().len(), 1);
    assert_eq!(audio_frames.lock().unwrap().len(), 1);
}

#[test]
fn test_ps_demuxer_feed_incrementally() {
    let mut muxer = PSMuxer::new();
    let sid = muxer.add_stream(PS_STREAM_AAC);
    let frame = [0xFF, 0xF1, 0x00, 0x50];

    let frames = Arc::new(Mutex::new(Vec::new()));
    let f = frames.clone();

    let mut demuxer = PSDemuxer::new();
    demuxer.set_on_frame(Box::new(
        move |_frame: &[u8], _cid: u8, _pts: u64, _dts: u64| {
            f.lock().unwrap().push(_frame.to_vec());
        },
    ));

    muxer.write(sid, &frame, 90000, 90000).unwrap();
    let ps_data = muxer.packet_data();

    let half = ps_data.len() / 2;
    match demuxer.input(&ps_data[..half]) {
        Ok(_) => {}
        Err(e) => {
            println!("{:?}", e);
        }
    }
    match demuxer.input(&ps_data[half..]) {
        Ok(_) => {}
        Err(e) => {
            println!("{:?}", e);
        }
    }
    demuxer.flush();

    assert_eq!(frames.lock().unwrap().len(), 1);
}

#[test]
fn test_ps_demuxer_unknown_stream_type() {
    let mut muxer = PSMuxer::new();
    let sid = muxer.add_stream(PS_STREAM_H264);
    let frame = [0x00, 0x00, 0x00, 0x01, 0x65, 0x88, 0x84, 0x00];

    let frames = Arc::new(Mutex::new(Vec::new()));
    let f = frames.clone();

    let mut demuxer = PSDemuxer::new();
    demuxer.set_on_frame(Box::new(
        move |_frame: &[u8], _cid: u8, _pts: u64, _dts: u64| {
            f.lock().unwrap().push(_frame.to_vec());
        },
    ));

    muxer.write(sid, &frame, 90000, 90000).unwrap();
    let ps_data = muxer.packet_data();

    demuxer.input(&ps_data).unwrap();
    demuxer.flush();

    let frames = frames.lock().unwrap();
    assert_eq!(frames[0], frame);
}

#[test]
fn test_ps_pack_header_decode() {
    let data = [
        0x00, 0x00, 0x01, 0xBA, 0x40, 0x01, 0x00, 0x01, 0x33, 0x44, 0xFF, 0xFF, 0xFF, 0xF1, 0xFF,
    ];
    let mut bs = BitStream::new(&data);
    let mut header = PSPackHeader::new();
    let result = header.decode(&mut bs);
    assert!(result.is_ok());
    assert!(!header.is_mpeg1);
}

#[test]
fn test_ps_system_header_decode() {
    let data = [
        0x00, 0x00, 0x01, 0xBB, 0x00, 0x0C, 0x80, 0x00, 0x40, 0x04, 0x00, 0x01, 0x00, 0x01, 0x00,
        0x00, 0x00, 0x01, 0x33, 0x44, 0xFF, 0x34,
    ];
    let mut bs = BitStream::new(&data);
    let mut header = SystemHeader::new();
    let result = header.decode(&mut bs);
    assert!(result.is_ok());
    assert_eq!(header.header_length, 12);
}

#[test]
fn test_ps_program_stream_map_decode() {
    let data = [
        0x00, 0x00, 0x01, 0xBC, 0x00, 0x0A, 0x40, 0x00, 0x00, 0x00, 0x00, 0x00, 0x03, 0x34, 0x81,
        0x00, 0x00,
    ];
    let mut bs = BitStream::new(&data);
    let mut psm = ProgramStreamMap::new();
    let result = psm.decode(&mut bs);
    assert!(result.is_ok());
    assert_eq!(psm.map_stream_id, 0xBC);
}

#[test]
fn test_common_pes_packet_decode() {
    let data = [
        0x00, 0x00, 0x01, 0xBD, 0x00, 0x0A, 0x31, 0x44, 0x81, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00,
    ];
    let mut bs = BitStream::new(&data);
    let mut cp = CommonPesPacket::new();
    let result = cp.decode(&mut bs);
    assert!(result.is_ok());
    assert_eq!(cp.stream_id, 0xBD);
    assert_eq!(cp.pes_packet_length, 0x000A);
}

#[test]
fn test_ps_demuxer_timestamp_parsing() {
    let mut muxer = PSMuxer::new();
    let sid = muxer.add_stream(PS_STREAM_H264);
    let frame = [0x00, 0x00, 0x00, 0x01, 0x65, 0x88, 0x84, 0x00];

    let pts_list = Arc::new(Mutex::new(Vec::new()));
    let dts_list = Arc::new(Mutex::new(Vec::new()));

    let pts_l = pts_list.clone();
    let dts_l = dts_list.clone();

    let mut demuxer = PSDemuxer::new();
    demuxer.set_on_frame(Box::new(
        move |_frame: &[u8], _cid: u8, _pts: u64, _dts: u64| {
            pts_l.lock().unwrap().push(_pts);
            dts_l.lock().unwrap().push(_dts);
        },
    ));

    let pts = 180000;
    let dts = 180000;
    muxer.write(sid, &frame, pts, dts).unwrap();
    let ps_data = muxer.packet_data();

    demuxer.input(&ps_data).unwrap();
    demuxer.flush();

    let pts_list = pts_list.lock().unwrap();
    let dts_list = dts_list.lock().unwrap();

    assert_eq!(pts_list.len(), 1);
    assert_eq!(pts_list[0], pts);
    assert_eq!(dts_list[0], dts);
}
