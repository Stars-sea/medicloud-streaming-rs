use std::path::Path;

use log::warn;
use tonic::{Request, Response, Status};

use crate::core::input;
use crate::livestream::livestream_server::Livestream;
use crate::livestream::{StartPullStreamRequest, StartPullStreamResponse};

#[derive(Debug, Default)]
pub struct LiveStreamService {}

#[tonic::async_trait]
impl Livestream for LiveStreamService {
    async fn start_pull_stream(
        &self,
        request: Request<StartPullStreamRequest>,
    ) -> Result<Response<StartPullStreamResponse>, Status> {
        let request = request.into_inner();

        let mut input = input::open_srt_input(
            request.url,
            request.timeout,
            request.latency,
            request.ffs,
            request.passphrase,
        )
        .map_err(|e| Status::internal(e.to_string()))?;

        // output.write_header()?;
        // for (stream, mut packet) in input.packets() {
        //     match output.stream(stream.index()) {
        //         Some(stream) => {
        //             let ostream = output.stream(stream.index()).unwrap();
        //             packet.rescale_ts(stream.time_base(), ostream.time_base());
        //             packet.write(&mut output)?;
        //         }
        //         None => {
        //             warn!("[PullStreamHandler] Failed to fetch packet")
        //         }
        //     }
        // }

        // output.write_trailer()?;

        let resp = StartPullStreamResponse {
            live_id: request.live_id,
            url: request.url,
            path: String::from(""),
            code: String::from(""),
        };

        Ok(Response::new(resp))
    }
}
