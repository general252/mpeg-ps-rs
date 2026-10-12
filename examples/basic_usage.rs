use mpeg_ps::*;
use pretty_hex::PrettyHex;
use std::sync::{Arc, Mutex};

fn main() {
    let mut muxer = PSMuxer::new();
    let demuxer = Arc::new(Mutex::new(PSDemuxer::new()));

    demuxer.lock().unwrap().set_on_frame(Box::new(
        move |frame: &[u8], cid: u8, _pts: u64, _dts: u64| {
            println!(
                ">>>>>>>>>> Frame received: cid:{} pts:{}, dts:{} frame: {:?}",
                cid,
                _pts,
                _dts,
                frame.hex_dump()
            );
        },
    ));

    let video_sid = muxer.add_stream(PS_STREAM_H264);
    println!("Added H264 video stream with SID: 0x{:02X}", video_sid);

    let audio_sid = muxer.add_stream(PS_STREAM_AAC);
    println!("Added AAC audio stream with SID: 0x{:02X}", audio_sid);

    let demuxer_clone = Arc::clone(&demuxer);
    muxer.set_on_packet(move |pkg| {
        println!("------------Packet received: {:?}", pkg.hex_dump());

        demuxer_clone.lock().unwrap().input(pkg).unwrap();
    });

    let mut h264_frame = vec![0x00, 0x00, 0x00, 0x01, 0x41, 0x9A, 0x21, 0x6C];
    h264_frame.extend_from_slice(&[0x00; 100]);

    let mut aac_frame = vec![0xFF, 0xF1, 0x50, 0x80, 0x00, 0x00];
    aac_frame.extend_from_slice(&[0x00; 50]);

    let pts: u64 = 0;
    let dts: u64 = 0;
    muxer.write(video_sid, &h264_frame, pts, dts).unwrap();
    muxer.write(audio_sid, &aac_frame, pts, dts).unwrap();

    println!("PS muxer example completed successfully!");

    demuxer.lock().unwrap().flush();
}
