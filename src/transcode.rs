use std::path::Path;
use std::path::PathBuf;

use crate::ffmpeg;
use crate::ffmpeg::{codec, filter, format, frame, media};

struct FFmpegTranscode {
    stream: usize,
    filter: filter::Graph,
    decoder: codec::decoder::Audio,
    encoder: codec::encoder::Audio,
    in_time_base: ffmpeg::Rational,
    out_time_base: ffmpeg::Rational,
    frame_count: usize,
}

impl FFmpegTranscode {
    pub fn new<P: AsRef<Path>>(
        ictx: &mut format::context::Input,
        octx: &mut format::context::Output,
        path: &P,
        filter_spec: &str,
    ) -> Result<Self, ffmpeg::Error> {
        let input = ictx
            .streams()
            .best(media::Type::Audio)
            .expect("could not find best audio stream");
        let mut decoder = input.codec().decoder().audio()?;

        // let codec = ffmpeg::encoder::find(octx.format().codec(path, media::Type::Audio))
        let codec = ffmpeg::encoder::find(codec::id::Id::MP3)
            .expect("failed to find encoder")
            .audio()?;
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
        encoder.set_bit_rate(320 * 1024);
        encoder.set_max_bit_rate(320 * 1024);
        // encoder.set_bit_rate(decoder.bit_rate());
        // encoder.set_max_bit_rate(decoder.max_bit_rate());

        encoder.set_time_base((1, decoder.rate() as i32));
        output.set_time_base((1, decoder.rate() as i32));

        let encoder = encoder.open_as(codec)?;
        output.set_parameters(&encoder);

        let filter = Self::build_filter(filter_spec, &decoder, &encoder)?;

        let in_time_base = decoder.time_base();
        let out_time_base = output.time_base();

        Ok(Self {
            stream: input.index(),
            filter,
            decoder,
            encoder,
            in_time_base,
            out_time_base,
            frame_count: 0,
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
            out.set_sample_rate(encoder.rate());
        }

        filter.output("in", 0)?.input("out", 0)?.parse(spec)?;
        filter.validate()?;

        println!("{}", filter.dump());

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
            self.frame_count += 1;
            // self.log_progress(f64::from(
            //     Rational(timestamp.unwrap_or(0) as i32, 1) * self.decoder.time_base(),
            // ));
            decoded.set_pts(timestamp);
            self.add_frame_to_filter(&decoded);
            self.get_and_process_filtered_frames(octx);
        }
        Ok(())
    }

    fn log_progress(&mut self, timestamp: f64) {
        // if !self.logging_enabled
        //     || (self.frame_count - self.last_log_frame_count < 100
        //         && self.last_log_time.elapsed().as_secs_f64() < 1.0)
        // {
        //     return;
        // }
        println!(
            // "time elpased: \t{:8.2}\tframe count: {:8}\ttimestamp: {:8.2}",
            "frame count: {:8}\ttimestamp: {:8.2}",
            // self.starting_time.elapsed().as_secs_f64(),
            self.frame_count,
            timestamp
        );
        // self.last_log_frame_count = self.frame_count;
        // self.last_log_time = Instant::now();
    }
}

pub trait Transcoder {
    // todo: add format options
    fn transcode(&self, input_path: PathBuf, output_path: PathBuf) -> Result<(), ffmpeg::Error>;
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
    fn transcode(&self, input_path: PathBuf, output_path: PathBuf) -> Result<(), ffmpeg::Error> {
        ffmpeg::init()?;

        let mut ictx = format::input(&input_path)?;
        let mut octx = format::output(&output_path)?;
        let mut transcoder = FFmpegTranscode::new(&mut ictx, &mut octx, &output_path, &"anull")?;

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
