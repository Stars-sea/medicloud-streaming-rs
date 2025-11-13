use crate::core::context::Context;
use crate::core::input::SrtInputContext;
use crate::core::output::TsOutputContext;
use crate::core::packet::Packet;
use crate::livestream::events::{OnSegmentComplete, SegmentCompleteTx, StopStreamRx};
use crate::settings::SegmentConfig;
use std::path::PathBuf;

fn should_segment(
    packet: &Packet,
    input_ctx: &impl Context,
    duration: f64,
    last_pts: &mut i64,
) -> bool {
    let current_pts = packet.pts().unwrap_or(0);
    let current_stream = input_ctx.stream(packet.stream_idx()).unwrap();
    if !current_stream.is_video_stream() || !packet.is_key_frame() {
        return false;
    }

    if (current_pts - *last_pts) as f64 * current_stream.time_base_f64() > duration {
        *last_pts = current_pts;
        return true;
    }

    false
}

pub(super) fn pull_srt_loop(
    segment_complete_tx: SegmentCompleteTx,
    mut stop_rx: StopStreamRx,
    live_id: String,
    srt_url: String,
    config: SegmentConfig,
) -> anyhow::Result<()> {
    let input_ctx = SrtInputContext::open(srt_url.as_str())?;
    let cache_dir = PathBuf::from(config.cache_dir).join(live_id.clone());

    let mut segment_id: u64 = 1;
    let mut output_ctx = TsOutputContext::create_segment(&cache_dir, &input_ctx, segment_id)?;

    let mut last_start_pts = 0;

    while !stop_rx.try_recv().is_ok_and(|id| id.live_id() == live_id) {
        let packet = Packet::alloc()?;
        if packet.read_safely(&input_ctx) == 0 {
            break;
        }

        if should_segment(
            &packet,
            &input_ctx,
            config.segment_time as f64,
            &mut last_start_pts,
        ) {
            output_ctx.release_and_close()?;
            segment_complete_tx.send(OnSegmentComplete::from_ctx(
                live_id.to_string(),
                &output_ctx,
            ))?;

            segment_id += 1;
            output_ctx = TsOutputContext::create_segment(&cache_dir, &input_ctx, segment_id)?;
        }

        packet.rescale_ts_for_ctx(&input_ctx, &output_ctx);
        packet.write(&output_ctx)?;
    }

    output_ctx.release_and_close()?;
    segment_complete_tx.send(OnSegmentComplete::from_ctx(
        live_id.to_string(),
        &output_ctx,
    ))?;

    Ok(())
}
