use anyhow::Result;
use ndarray::parallel::prelude::*;
use ndarray::prelude::*;
use ndarray::{indices, Array, IntoDimension, NdIndex, RemoveAxis, Zip};
use num::pow::pow;
use num::traits::{Float, FloatConst, FromPrimitive, NumCast, Signed, Zero};
use rodio::{source::Source, Decoder};
use rustfft::{
    algorithm::BluesteinsAlgorithm, algorithm::GoodThomasAlgorithm, algorithm::MixedRadix,
    algorithm::RadersAlgorithm, algorithm::Radix4, num_complex::Complex, Fft, FftDirection, FftNum,
};
use std::fs::File;
use std::io::BufReader;
use std::path::PathBuf;
use std::sync::Arc;
use std::thread;
use std::time::Instant;

pub fn get_source_from_file(
    path: &PathBuf,
) -> Result<(Box<dyn rodio::Source<Item = f32> + Send>, u32, u16)> {
    let file = BufReader::new(File::open(path)?);
    let source = Decoder::new(file)?.convert_samples();
    let sample_rate = source.sample_rate();
    let nchannels = source.channels();
    Ok((Box::new(source), sample_rate, nchannels))
}

pub fn padded_fft<T>(data: &Array1<T>, fft: Arc<dyn Fft<T>>, size: usize) -> Array1<Complex<T>>
where
    T: FromPrimitive + Signed + Zero + FftNum + Clone + std::fmt::Debug + Sync + Send + 'static,
{
    let original_size = data.len();
    let mut out = data.to_owned();
    out.append(Axis(0), Array::zeros(size - original_size).view())
        .unwrap();
    let mut out: Array1<Complex<T>> = out.mapv(|v| Complex {
        re: v,
        im: T::zero(),
    });
    match out.as_slice_mut() {
        Some(fft_out) => {
            fft.process(fft_out);
            out
        }
        None => {
            let mut fft_out: Vec<Complex<T>> = out.to_vec();
            fft.process(&mut fft_out);
            Array1::from_iter(fft_out)
        }
    }
}

pub fn fftconvolve<T>(volume: &Array1<T>, kernel: &Array1<T>) -> Array1<T>
where
    T: FromPrimitive + Signed + Float + FloatConst + std::fmt::Debug + Sync + Send + 'static,
{
    let size = volume.len() + kernel.len() - 1;
    let size2: usize = NumCast::from((size as f32).log2().ceil()).unwrap();
    let size2: usize = pow(2, size2);
    let fft = Arc::new(Radix4::new(size2, FftDirection::Forward));
    let ifft = Radix4::new(size2, FftDirection::Inverse);

    let vfft = fft.clone();
    let volume_size = volume.len();
    let volume = volume.to_owned();
    let volume_fft_handle = thread::spawn(move || padded_fft(&volume, vfft, size2));

    let kfft = fft.clone();
    let kernel_size = kernel.len();
    let kernel = kernel.to_owned();
    let kernel_fft_handle = thread::spawn(move || padded_fft(&kernel, kfft, size2));

    let volume_fft = volume_fft_handle.join().unwrap();
    let kernel_fft = kernel_fft_handle.join().unwrap();

    let start = Instant::now();
    let mut ret_fft = (volume_fft * kernel_fft);
    let ret_fft = match ret_fft.as_slice_mut() {
        Some(ret_fft_slice) => {
            ifft.process(ret_fft_slice);
            ret_fft
        }
        None => {
            let mut ret_fft = ret_fft.to_vec();
            ifft.process(&mut ret_fft);
            Array1::from(ret_fft)
        }
    };
    let valid_len = volume_size - kernel_size + 1;
    let valid_start = (size - valid_len) / 2;
    let valid_end = valid_start + valid_len;
    let scale = T::from(1.0 / ret_fft.len() as f32).unwrap();
    let mut ret_fft = ret_fft
        .slice(s![valid_start..valid_end])
        .map(|v| T::from(v.scale(scale).norm()).unwrap());
    assert!(ret_fft.len() == valid_len);
    ret_fft
}

pub fn correlate<T>(volume: &Array1<T>, kernel: &Array1<T>) -> (Array1<T>, (usize, T))
where
    T: FromPrimitive + Signed + Float + FloatConst + std::fmt::Debug + Sync + Send + 'static,
{
    let start = Instant::now();
    let kernel = kernel.slice(s![..;-1]).to_owned();
    let out = fftconvolve(volume, &kernel);

    let mut argmax: Option<(usize, &T)> = None;
    for new in out.indexed_iter() {
        argmax = match argmax {
            Some(old) => {
                if old.1 > new.1 {
                    Some(old)
                } else {
                    Some(new)
                }
            }
            None => Some(new),
        };
    }
    let peak = argmax.unwrap();
    let peak = (peak.0, *peak.1);
    (out, peak)
}

#[cfg(test)]
mod tests {
    use crate::matching::correlate::correlate;
    use approx::assert_abs_diff_eq;
    use ndarray::parallel::prelude::*;
    use ndarray::prelude::*;
    use ndarray::{Array, Zip};
    use rodio::{source::Source, Decoder};
    use std::fs::File;
    use std::io::BufReader;
    use std::io::{Cursor, Read, Seek, SeekFrom, Write};
    use std::path::PathBuf;
    use std::time::Instant;

    #[test]
    fn test_correlation() {
        let within = Array::from_iter(0..501).mapv(|v| v as f64);
        let find = Array::from_iter(100..201).mapv(|v| v as f64);
        let (correlation, peak) = correlate(&within, &find);
        assert_abs_diff_eq!(peak.0 as f32, 400.0f32, epsilon = 1.0);
    }

    // #[test]
    // fn test_correlation_for_file() {
    //     let start = Instant::now();
    //     let total_bytes = include_bytes!("../../experimental/audio-samples/muse_uprising.mp3");
    //     let total = Cursor::new(total_bytes.as_ref());
    //     let total_source = Decoder::new(total).unwrap().convert_samples::<f32>();

    //     let preview_bytes = include_bytes!("../../experimental/audio-samples/muse_preview.mp3");
    //     let preview = Cursor::new(preview_bytes.as_ref());
    //     let preview_source = Decoder::new(preview).unwrap().convert_samples::<f32>();

    //     assert!(total_source.sample_rate() == preview_source.sample_rate());
    //     println!("decoded in {:?}", start.elapsed());

    //     let start = Instant::now();
    //     let within_sample_rate = total_source.sample_rate();
    //     let within_channels = total_source.channels();
    //     let within: Vec<f32> = total_source.collect();
    //     let (r, c) = (
    //         within.len() / (within_channels as usize),
    //         within_channels as usize,
    //     );
    //     let mut within: Array2<f32> = Array::from_iter(within).into_shape([r, c]).unwrap();
    //     within.par_mapv_inplace(|v| v.abs());
    //     let within = Zip::from(within.axis_iter(Axis(0)))
    //         .par_map_collect(|row| row.iter().fold(0f32, |acc, v| acc.max(*v)));
    //     println!("within done in {:?}", start.elapsed());

    //     let start = Instant::now();
    //     let find_sample_rate = preview_source.sample_rate();
    //     let find_channels = preview_source.channels();
    //     let find: Vec<f32> = preview_source.collect();
    //     let (r, c) = (
    //         find.len() / (find_channels as usize),
    //         find_channels as usize,
    //     );
    //     let mut find: Array2<f32> = Array::from_iter(find).into_shape([r, c]).unwrap();
    //     find.par_mapv_inplace(|v| v.abs());
    //     let find = Zip::from(find.axis_iter(Axis(0)))
    //         .par_map_collect(|row| row.iter().fold(0f32, |acc, v| acc.max(*v)));
    //     println!("find done in {:?}", start.elapsed());

    //     println!("within: {} seconds", within.len() as u32 / find_sample_rate);
    //     println!("find: {} seconds", find.len() as u32 / find_sample_rate);
    //     let (correlation, peak) = correlate(&within, &find);
    //     let offset = peak.0 as f32 / find_sample_rate as f32;
    //     assert_abs_diff_eq!(offset, 147.0, epsilon = 1.0);
    // }
}
