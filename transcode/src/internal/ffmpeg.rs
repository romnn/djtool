#![allow(
    clippy::cast_possible_wrap,
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
)]

use crate::{Codec, Error, ProgressHandlerFunc, TranscodeProgress, Transcoder, TranscoderOptions};
use djtool_ffmpeg as ffmpeg;
use std::path::Path;
use std::time::{Duration, Instant};

impl From<Codec> for ffmpeg::codec::id::Id {
    fn from(codec: Codec) -> Self {
        match codec {
            Codec::MP3 => Self::MP3,
            Codec::PCM => Self::PCM_S16LE,
        }
    }
}

struct FFmpegTranscode<'a> {
    stream: usize,
    filter: ffmpeg::filter::Graph,
    decoder: ffmpeg::codec::decoder::Audio,
    encoder: ffmpeg::codec::encoder::Audio,
    in_time_base: ffmpeg::Rational,
    out_time_base: ffmpeg::Rational,
    duration: u64,
    total_frames: u64,
    frame: u64,
    started: Instant,
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
            .best(ffmpeg::media::Type::Audio)
            .expect("could not find best audio stream");
        // let duration = input.duration();
        let mut decoder = input.codec().decoder().audio()?;
        // let duration = decoder.duration();

        let encoder = match options.and_then(|o| o.codec) {
            Some(requested_codec) => requested_codec.into(),
            None => octx.format().codec(path, ffmpeg::media::Type::Audio),
        };
        // println!("chosen encoder: {:?}", encoder);

        let codec = ffmpeg::encoder::find(encoder)
            .ok_or(ffmpeg::error::Error::EncoderNotFound)
            .and_then(djtool_ffmpeg::Codec::audio)?;

        let global = octx
            .format()
            .flags()
            .contains(ffmpeg::format::Flags::GLOBAL_HEADER);

        decoder.set_parameters(input.parameters())?;

        let mut output = octx.add_stream(codec)?;
        let mut encoder = output.codec().encoder().audio()?;

        let channel_layout = codec
            .channel_layouts()
            .map_or(ffmpeg::channel_layout::ChannelLayout::STEREO, |cls| {
                cls.best(decoder.channel_layout().channels())
            });

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

        let mut filters = Vec::new();
        if let Some(options) = options {
            if options.loudness_normalize {
                filters.push("loudnorm");
            }
        }
        let filter_spec = if filters.is_empty() {
            "anull".to_string()
        } else {
            filters.join(",")
        };
        let filter = Self::build_filter(&filter_spec, &decoder, &encoder)?;

        let in_time_base = decoder.time_base();
        let out_time_base = output.time_base();
        let started = Instant::now();

        // this does not work for audio streams
        let total_frames = input.duration() as f64 * f64::from(input.rate());
        Ok(Self {
            stream: input.index(),
            filter,
            decoder,
            encoder,
            in_time_base,
            out_time_base,
            duration: input.duration().unsigned_abs(),
            total_frames: total_frames as u64,
            started,
            frame: 0,
            progress_handler,
        })
    }

    fn build_filter(
        spec: &str,
        decoder: &ffmpeg::codec::decoder::Audio,
        encoder: &ffmpeg::codec::encoder::Audio,
    ) -> Result<ffmpeg::filter::Graph, ffmpeg::Error> {
        let mut filter = ffmpeg::filter::Graph::new();

        let args = format!(
            "time_base={}:sample_rate={}:sample_fmt={}:channel_layout=0x{:x}",
            decoder.time_base(),
            decoder.rate(),
            decoder.format().name(),
            decoder.channel_layout().bits()
        );

        filter.add(
            &ffmpeg::filter::find("abuffer").ok_or(ffmpeg::Error::FilterNotFound)?,
            "in",
            &args,
        )?;
        filter.add(
            &ffmpeg::filter::find("abuffersink").ok_or(ffmpeg::Error::FilterNotFound)?,
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
        octx: &mut ffmpeg::format::context::Output,
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
        octx: &mut ffmpeg::format::context::Output,
    ) -> Result<(), ffmpeg::Error> {
        let mut filtered = ffmpeg::frame::Audio::empty();
        loop {
            let mut f = self
                .filter
                .get("out")
                .ok_or(ffmpeg::Error::FilterNotFound)?;
            match f.sink().frame(&mut filtered) {
                Ok(_) => {
                    self.send_frame_to_encoder(&filtered)?;
                    self.receive_and_process_encoded_packets(octx)?;
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
        octx: &mut ffmpeg::format::context::Output,
    ) -> Result<(), ffmpeg::Error> {
        let mut decoded = ffmpeg::frame::Audio::empty();
        while self.decoder.receive_frame(&mut decoded).is_ok() {
            let timestamp = decoded.timestamp();
            decoded.set_pts(timestamp);
            self.frame += 1;

            let duration = self.to_duration(self.duration as f64);
            let timestamp = self.to_duration(timestamp.unwrap_or(0) as f64);
            (self.progress_handler)(TranscodeProgress {
                elapsed: self.started.elapsed(),
                frame: self.frame,
                total_frames: self.total_frames,
                duration,
                timestamp,
            });
            self.add_frame_to_filter(&decoded)?;
            self.get_and_process_filtered_frames(octx)?;
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
}

#[derive(Default)]
pub struct FFmpegTranscoder {}

impl FFmpegTranscoder {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

impl FFmpegTranscoder {
    /// Transcode input file to output path
    ///
    /// # Errors
    /// If an ffmpeg error occurs during transcoding.
    pub fn transcode(
        &self,
        input_path: &Path,
        output_path: &Path,
        options: Option<&TranscoderOptions>,
        progress_handler: &mut ProgressHandlerFunc,
    ) -> Result<(), ffmpeg::Error> {
        ffmpeg::init()?;

        let mut ictx = ffmpeg::format::input(&input_path)?;
        let mut octx = ffmpeg::format::output(&output_path)?;
        let mut transcoder = FFmpegTranscode::new(
            &mut ictx,
            &mut octx,
            &output_path,
            options,
            progress_handler,
        )?;

        octx.set_metadata(ictx.metadata().to_owned());
        octx.write_header()?;

        for (stream, mut packet) in ictx.packets() {
            if stream.index() == transcoder.stream {
                packet.rescale_ts(stream.time_base(), transcoder.in_time_base);
                transcoder.send_packet_to_decoder(&packet)?;
                transcoder.receive_and_process_decoded_frames(&mut octx)?;
            }
        }

        transcoder.send_eof_to_decoder()?;
        transcoder.receive_and_process_decoded_frames(&mut octx)?;

        transcoder.flush_filter()?;
        transcoder.get_and_process_filtered_frames(&mut octx)?;

        transcoder.send_eof_to_encoder()?;
        transcoder.receive_and_process_encoded_packets(&mut octx)?;

        octx.write_trailer()?;
        Ok(())
    }
}

impl Transcoder for FFmpegTranscoder {
    fn transcode_blocking(
        &self,
        input_path: &Path,
        output_path: &Path,
        options: Option<&TranscoderOptions>,
        progress_handler: &mut ProgressHandlerFunc,
    ) -> Result<(), Error> {
        self.transcode(input_path, output_path, options, progress_handler)
            .map_err(|err| Error::Custom(err.into()))
    }
}
