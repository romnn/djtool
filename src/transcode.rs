use std::path::{Path, PathBuf};

use djtool_ffmpeg as ffmpeg;
// ,  codec, filter, format, frame, media};
use std::time::{Duration, Instant};

#[derive(Debug)]
pub struct TranscodeProgress {
    pub elapsed: Duration,
    pub frame: usize,
    pub total_frames: usize,
    pub duration: Duration,
    pub timestamp: Duration,
}

pub type ProgressHandlerFunc = dyn FnMut(TranscodeProgress) -> ();

#[derive(Debug, Clone)]
pub enum Codec {
    MP3,
    PCM,
    // todo: add more codecs in the future
}

impl Codec {
    fn codec(&self) -> ffmpeg::codec::id::Id {
        match self {
            Codec::MP3 => ffmpeg::codec::id::Id::MP3,
            Codec::PCM => ffmpeg::codec::id::Id::PCM_S16LE,
        }
    }
}

#[derive(Default, Debug, Clone)]
pub struct TranscoderOptions<'a> {
    pub codec: Option<Codec>,
    pub bitrate_kbps: Option<usize>,
    pub sample_rate: Option<usize>,
    pub filter_spec: Option<&'a str>,
}

impl TranscoderOptions<'_> {
    pub fn mp3() -> Self {
        Self {
            codec: Some(ffmpeg::Codec::MP3),
            bitrate_kbps: Some(192),
            sample_rate: None,
            filter_spec: Some("loudnorm"),
        }
    }

    pub fn matching() -> Self {
        Self {
            codec: Some(ffmpeg::Codec::PCM),
            bitrate_kbps: None,
            // most importantly, we resample
            sample_rate: Some(22_050),
            filter_spec: None,
        }
    }
}

pub trait Transcoder {
    fn transcode_blocking(
        &self,
        input_path: &PathBuf,
        output_path: &PathBuf,
        options: Option<&TranscoderOptions>,
        progess_handler: &mut ProgressHandlerFunc,
    ) -> Result<(), ffmpeg::Error>;
}

struct FFmpegTranscode<'a> {
    stream: usize,
    filter: filter::Graph,
    decoder: ffmpeg::codec::decoder::Audio,
    encoder: ffmpeg::codec::encoder::Audio,
    in_time_base: ffmpeg::Rational,
    out_time_base: ffmpeg::Rational,
    duration: usize,
    total_frames: usize,
    started: Instant,
    frame: usize,
    progress_handler: &'a mut ProgressHandlerFunc,
}

impl<'a> FFmpegTranscode<'a> {
    pub fn new<P: AsRef<Path>>(
        ictx: &mut ffmpeg::format::context::Input,
        octx: &mut ffmpeg::format::context::Output,
        path: &P,
        options: Option<&TranscoderOptions>,
        progress_handler: &'a mut ProgressHandlerFunc,
    ) -> Result<Self, ffmpeg::Error> {
        // println!("options: {:?}", options);
        let input = ictx
            .streams()
            .best(media::Type::Audio)
            .expect("could not find best audio stream");
        // let duration = input.duration();
        let mut decoder = input.codec().decoder().audio()?;
        // let duration = decoder.duration();

        let encoder = match options.and_then(|o| o.codec.as_ref()) {
            Some(requested_codec) => requested_codec.codec(),
            None => octx.format().codec(path, media::Type::Audio),
        };
        // println!("chosen encoder: {:?}", encoder);

        let codec = ffmpeg::encoder::find(encoder)
            .ok_or(ffmpeg::error::Error::EncoderNotFound)
            .and_then(|c| c.audio())?;

        let global = octx
            .format()
            .flags()
            .contains(ffmpeg::format::flag::Flags::GLOBAL_HEADER);

        decoder.set_parameters(input.parameters())?;

        let mut output = octx.add_stream(codec)?;
        let mut encoder = output.codec().encoder().audio()?;

        let channel_layout = codec
            .channel_layouts()
            .map(|cls| cls.best(decoder.channel_layout().channels()))
            .unwrap_or(ffmpeg::channel_layout::ChannelLayout::STEREO);

        if global {
            encoder.set_flags(ffmpeg::codec::flag::Flags::GLOBAL_HEADER);
        }

        encoder.set_rate(decoder.rate() as i32);
        encoder.set_channel_layout(channel_layout);
        encoder.set_channels(channel_layout.channels());
        if let Some(format) = codec
            .formats()
            .ok_or(ffmpeg::error::Error::EncoderNotFound)?
            .next()
        {
            encoder.set_format(format);
        }

        let (bitrate, max_bitrate) = match options.and_then(|o| o.bitrate_kbps.as_ref()) {
            Some(kbps) => (kbps * 1024, kbps * 1024),
            None => (decoder.bit_rate(), decoder.max_bit_rate()),
        };
        encoder.set_bit_rate(bitrate);
        encoder.set_max_bit_rate(max_bitrate);

        encoder.set_time_base((1, decoder.rate() as i32));
        output.set_time_base((1, decoder.rate() as i32));

        let encoder = encoder.open_as(codec)?;
        output.set_parameters(&encoder);

        let filter = Self::build_filter(
            options.and_then(|o| o.filter_spec).unwrap_or(&"anull"),
            &decoder,
            &encoder,
        )?;

        let in_time_base = decoder.time_base();
        let out_time_base = output.time_base();
        let started = Instant::now();

        Ok(Self {
            stream: input.index(),
            filter,
            decoder,
            encoder,
            in_time_base,
            out_time_base,
            duration: input.duration() as usize,
            // this does not work for audio streams
            total_frames: (input.duration() as f64 * f64::from(input.rate())) as usize,
            started,
            frame: 0,
            progress_handler,
        })
    }

    fn build_filter(
        spec: &str,
        decoder: &codec::decoder::Audio,
        encoder: &codec::encoder::Audio,
    ) -> Result<filter::Graph, ffmpeg::Error> {
        let mut filter = filter::Graph::new();

        let args = format!(
            "time_base={}:sample_rate={}:sample_fmt={}:channel_layout=0x{:x}",
            decoder.time_base(),
            decoder.rate(),
            decoder.format().name(),
            decoder.channel_layout().bits()
        );

        filter.add(
            &filter::find("abuffer").ok_or(ffmpeg::Error::FilterNotFound)?,
            "in",
            &args,
        )?;
        filter.add(
            &filter::find("abuffersink").ok_or(ffmpeg::Error::FilterNotFound)?,
            "out",
            "",
        )?;

        {
            let mut out = filter.get("out").ok_or(ffmpeg::Error::FilterNotFound)?;

            out.set_sample_format(encoder.format());
            out.set_channel_layout(encoder.channel_layout());
            // todo: sample rate here
            out.set_sample_rate(encoder.rate());
        }

        filter.output("in", 0)?.input("out", 0)?.parse(spec)?;
        filter.validate()?;

        // println!("{}", filter.dump());

        if let Some(codec) = encoder.codec() {
            if !codec
                .capabilities()
                .contains(ffmpeg::codec::capabilities::Capabilities::VARIABLE_FRAME_SIZE)
            {
                let mut f = filter.get("out").ok_or(ffmpeg::Error::FilterNotFound)?;
                f.sink().set_frame_size(encoder.frame_size());
            }
        }

        Ok(filter)
    }

    fn send_frame_to_encoder(&mut self, frame: &ffmpeg::Frame) -> Result<(), ffmpeg::Error> {
        self.encoder.send_frame(frame)
    }

    fn send_eof_to_encoder(&mut self) -> Result<(), ffmpeg::Error> {
        self.encoder.send_eof()
    }

    fn receive_and_process_encoded_packets(
        &mut self,
        octx: &mut format::context::Output,
    ) -> Result<(), ffmpeg::Error> {
        let mut encoded = ffmpeg::Packet::empty();
        while self.encoder.receive_packet(&mut encoded).is_ok() {
            encoded.set_stream(0);
            encoded.rescale_ts(self.in_time_base, self.out_time_base);
            encoded.write_interleaved(octx)?;
        }
        Ok(())
    }

    fn add_frame_to_filter(&mut self, frame: &ffmpeg::Frame) -> Result<(), ffmpeg::Error> {
        let mut f = self.filter.get("in").ok_or(ffmpeg::Error::FilterNotFound)?;
        f.source().add(frame)
    }

    fn flush_filter(&mut self) -> Result<(), ffmpeg::Error> {
        let mut f = self.filter.get("in").ok_or(ffmpeg::Error::FilterNotFound)?;
        f.source().flush()
    }

    fn get_and_process_filtered_frames(
        &mut self,
        octx: &mut format::context::Output,
    ) -> Result<(), ffmpeg::Error> {
        let mut filtered = frame::Audio::empty();
        loop {
            let mut f = self
                .filter
                .get("out")
                .ok_or(ffmpeg::Error::FilterNotFound)?;
            match f.sink().frame(&mut filtered) {
                Ok(_) => {
                    self.send_frame_to_encoder(&filtered);
                    self.receive_and_process_encoded_packets(octx);
                }
                Err(_) => break,
            };
        }
        Ok(())
    }

    fn send_packet_to_decoder(&mut self, packet: &ffmpeg::Packet) -> Result<(), ffmpeg::Error> {
        self.decoder.send_packet(packet)
    }

    fn send_eof_to_decoder(&mut self) -> Result<(), ffmpeg::Error> {
        self.decoder.send_eof()
    }

    fn receive_and_process_decoded_frames(
        &mut self,
        octx: &mut format::context::Output,
    ) -> Result<(), ffmpeg::Error> {
        let mut decoded = frame::Audio::empty();
        while self.decoder.receive_frame(&mut decoded).is_ok() {
            let timestamp = decoded.timestamp();
            decoded.set_pts(timestamp);
            self.frame += 1;

            // debug!(timestamp);
            // debug!(self.decoder.time_base());
            // check if timestamp is valid

            let duration = self.to_duration(self.duration as f64);
            let timestamp = self.to_duration(timestamp.unwrap_or(0) as f64);
            (self.progress_handler)(TranscodeProgress {
                elapsed: self.started.elapsed(),
                frame: self.frame,
                total_frames: self.total_frames,
                duration,
                // Duration::from_secs_f64(
                //     self.duration as f64 * f64::from(self.decoder.time_base()),
                // ),
                timestamp,
                // : self.to_duration(&(timestamp.unwrap_or(0) as f64)),
                // timestamp: Duration::from_secs_f64(),
            });
            self.add_frame_to_filter(&decoded);
            self.get_and_process_filtered_frames(octx);
        }
        Ok(())
    }

    fn to_duration(&self, sample: f64) -> Duration {
        let duration = sample * f64::from(self.decoder.time_base());
        if 0f64 <= duration && duration <= Duration::MAX.as_secs_f64() {
            Duration::from_secs_f64(duration)
        } else {
            Duration::ZERO
        }
    }

    // fn timestamp(&self) -> Duration {
    //     let timestamp = timestamp.unwrap_or(0) as f64 * f64::from(self.decoder.time_base());
    //     if 0f64 <= timestamp && timestamp <= Duration::MAX.as_secs_f64() {
    //         Duration::from_secs_f64(timestamp)
    //     } else {
    //         Duration::ZERO
    //     }
    // }

    // fn log_progress(&mut self, timestamp: f64) {
    //     // if !self.logging_enabled
    //     //     || (self.frame_count - self.last_log_frame_count < 100
    //     //         && self.last_log_time.elapsed().as_secs_f64() < 1.0)
    //     // {
    //     //     return;
    //     // }
    //     println!(
    //         // "time elpased: \t{:8.2}\tframe count: {:8}\ttimestamp: {:8.2}",
    //         "frame count: {:8}\ttimestamp: {:8.2}",
    //         // self.starting_time.elapsed().as_secs_f64(),
    //         self.frame_count,
    //         timestamp
    //     );
    //     // self.last_log_frame_count = self.frame_count;
    //     // self.last_log_time = Instant::now();
    // }
}

pub struct FFmpegTranscoder {}

impl FFmpegTranscoder {
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for FFmpegTranscoder {
    fn default() -> Self {
        Self {}
    }
}

impl Transcoder for FFmpegTranscoder {
    fn transcode_blocking(
        &self,
        input_path: &PathBuf,
        output_path: &PathBuf,
        options: Option<&TranscoderOptions>,
        progess_handler: &mut ProgressHandlerFunc,
    ) -> Result<(), ffmpeg::Error> {
        ffmpeg::init()?;

        let mut ictx = format::input(&input_path)?;
        let mut octx = format::output(&output_path)?;
        let mut transcoder =
            FFmpegTranscode::new(&mut ictx, &mut octx, &output_path, options, progess_handler)?;

        octx.set_metadata(ictx.metadata().to_owned());
        octx.write_header()?;

        for (stream, mut packet) in ictx.packets() {
            if stream.index() == transcoder.stream {
                packet.rescale_ts(stream.time_base(), transcoder.in_time_base);
                transcoder.send_packet_to_decoder(&packet);
                transcoder.receive_and_process_decoded_frames(&mut octx);
            }
        }

        transcoder.send_eof_to_decoder();
        transcoder.receive_and_process_decoded_frames(&mut octx);

        transcoder.flush_filter();
        transcoder.get_and_process_filtered_frames(&mut octx);

        transcoder.send_eof_to_encoder();
        transcoder.receive_and_process_encoded_packets(&mut octx);

        octx.write_trailer()?;
        Ok(())
    }
}
