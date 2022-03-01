import argparse

import time
from pathlib import Path
import librosa
import numpy as np
from scipy import signal
from scipy import linalg, fft as sp_fft
import matplotlib.pyplot as plt

EXPERIMENT_DIR = Path(__file__).parent
AUDIO_SAMPLES = EXPERIMENT_DIR / "audio-samples"


def _centered(arr, newshape):
    # Return the center newshape portion of the array.
    newshape = np.asarray(newshape)
    currshape = np.array(arr.shape)
    startind = (currshape - newshape) // 2
    endind = startind + newshape
    myslice = [slice(startind[k], endind[k]) for k in range(len(endind))]
    # print(myslice)
    return arr[tuple(myslice)]


def _init_nd_shape_and_axes(x, shape, axes):
    """Handles shape and axes arguments for nd transforms"""
    noshape = shape is None
    noaxes = axes is None

    if not noaxes:
        axes = _iterable_of_int(axes, "axes")
        axes = [a + x.ndim if a < 0 else a for a in axes]

        if any(a >= x.ndim or a < 0 for a in axes):
            raise ValueError("axes exceeds dimensionality of input")
        if len(set(axes)) != len(axes):
            raise ValueError("all axes must be unique")

    if not noshape:
        shape = _iterable_of_int(shape, "shape")

        if axes and len(axes) != len(shape):
            raise ValueError(
                "when given, axes and shape arguments" " have to be of the same length"
            )
        if noaxes:
            if len(shape) > x.ndim:
                raise ValueError("shape requires more axes than are present")
            axes = range(x.ndim - len(shape), x.ndim)

        shape = [x.shape[a] if s == -1 else s for s, a in zip(shape, axes)]
    elif noaxes:
        shape = list(x.shape)
        axes = range(x.ndim)
    else:
        shape = [x.shape[a] for a in axes]

    if any(s < 1 for s in shape):
        raise ValueError("invalid number of data points ({0}) specified".format(shape))

    return shape, axes


def _reverse_and_conj(x):
    reverse = (slice(None, None, -1),) * x.ndim
    return x[reverse].conj()


def _inputs_swap_needed(shape1, shape2, axes=None):
    if not shape1:
        return False

    if axes is None:
        axes = range(len(shape1))

    ok1 = all(shape1[i] >= shape2[i] for i in axes)
    ok2 = all(shape2[i] >= shape1[i] for i in axes)

    if not (ok1 or ok2):
        raise ValueError(
            "For 'valid' mode, one must be at least "
            "as large as the other in every dimension"
        )

    return not ok1


def _init_freq_conv_axes(in1, in2, axes=None, sorted_axes=False):
    s1 = in1.shape
    s2 = in2.shape
    noaxes = axes is None

    _, axes = _init_nd_shape_and_axes(in1, axes=axes, shape=None)

    if not noaxes and not len(axes):
        raise ValueError("when provided, axes cannot be empty")

    # Axes of length 1 can rely on broadcasting rules for multipy,
    # no fft needed.
    axes = [a for a in axes if s1[a] != 1 and s2[a] != 1]

    if sorted_axes:
        axes.sort()

    if not all(
        s1[a] == s2[a] or s1[a] == 1 or s2[a] == 1
        for a in range(in1.ndim)
        if a not in axes
    ):
        raise ValueError(
            "incompatible shapes for in1 and in2:" " {0} and {1}".format(s1, s2)
        )

    # Check that input sizes are compatible with 'valid' mode.
    if _inputs_swap_needed(s1, s2, axes=axes):
        # Convolution is commutative; order doesn't have any effect on output.
        in1, in2 = in2, in1

    return in1, in2, axes


def _freq_domain_conv(in1, in2, axes, shape, calc_fast_len=False):
    if not len(axes):
        return in1 * in2

    complex_result = in1.dtype.kind == "c" or in2.dtype.kind == "c"

    if calc_fast_len:
        # Speed up FFT by padding to optimal size.
        fshape = [sp_fft.next_fast_len(shape[a], not complex_result) for a in axes]
    else:
        fshape = shape

    # print("fast shape:", fshape)
    if not complex_result:
        fft, ifft = sp_fft.rfftn, sp_fft.irfftn
    else:
        fft, ifft = sp_fft.fftn, sp_fft.ifftn

    # compute the fft
    start = time.time()
    sp1 = fft(in1, fshape, axes=axes)
    sp2 = fft(in2, fshape, axes=axes)
    # print("sp1", np.abs(sp1))
    # print("sp2", np.abs(sp2))
    # print(fshape, sp1.shape, sp2.shape)
    print("fft took", time.time() - start)

    # print("ret fft", np.abs(sp1 * sp2))
    start = time.time()
    ret = ifft(sp1 * sp2, fshape, axes=axes)
    print("ret ifft", np.abs(ret))
    print("ret ifft took", time.time() - start)

    if calc_fast_len:
        # cut off the padding again
        fslice = tuple([slice(sz) for sz in shape])
        ret = ret[fslice]

    return ret


def _apply_conv_mode(ret, s1, s2, axes):
    # print(axes)
    # print(list(range(ret.ndim)))
    shape_valid = [
        ret.shape[a] if a not in axes else s1[a] - s2[a] + 1 for a in range(ret.ndim)
    ]
    # print("ret shape", ret.shape)
    # print("ret valid shape", shape_valid)
    return _centered(ret, shape_valid).copy()


def fftconvolve(in1, in2):
    in1 = np.asarray(in1)
    in2 = np.asarray(in2)

    in1, in2, axes = _init_freq_conv_axes(in1, in2, sorted_axes=False)
    # print(in1.shape)
    # print(in2.shape)
    # print(axes)

    s1 = in1.shape
    s2 = in2.shape

    shape = [
        max((s1[i], s2[i])) if i not in axes else s1[i] + s2[i] - 1
        for i in range(in1.ndim)
    ]
    # print("shape:", shape)

    ret = _freq_domain_conv(in1, in2, axes, shape, calc_fast_len=False)

    # print("ret before:", ret)
    ret = _apply_conv_mode(ret, s1, s2, axes)
    # print("ret after:", ret)
    return ret


def convolve(in1, in2):
    volume = np.asarray(in1)
    kernel = np.asarray(in2)
    out = fftconvolve(volume, kernel)
    result_type = np.result_type(volume, kernel)
    if result_type.kind in {"u", "i"}:
        out = np.around(out)
    return out.astype(result_type)


def correlate(in1, in2):
    # print("before", in2)
    # print("reversed", _reverse_and_conj(in2))
    start = time.time()
    return convolve(in1, _reverse_and_conj(in2))
    print("total", time.time() - start)


def find_offset(within_file, find_file):
    start = time.time()
    sr = None
    # sr = 22_050
    # sr = 10
    sr_within = sr
    y_within, sr_within = librosa.load(str(within_file), sr=sr)
    y_find, _ = librosa.load(str(find_file), sr=sr_within)
    # y_within = np.linspace(0, 500, 501)
    # y_find = np.linspace(100, 200, 101)
    print("y within samples:", len(y_within))
    print("y find samples:", len(y_find))

    print("loaded files within ", time.time() - start)

    print(len(y_within) / sr_within)
    print(len(y_find) / sr_within)
    assert len(y_within) > len(y_find)

    start = time.time()
    c = correlate(y_within, y_find)
    print("correlated in ", time.time() - start)

    peak = np.argmax(c)
    print("peak", peak)
    offset = round(peak / sr_within, 2)
    print("offset", offset)

    fig, ax = plt.subplots()
    ax.plot(np.arange(0, len(c)) / sr_within, c)
    fig.savefig("cross-correlation-%s-%s.png" % (within_file.name, find_file.name))
    return offset


def cli():
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--find-offset-of",
        metavar="audio file",
        type=str,
        help="Find the offset of file",
    )
    parser.add_argument("--within", metavar="audio file", type=str, help="Within file")
    parser.add_argument(
        "--window",
        metavar="seconds",
        type=int,
        default=10,
        help="Only use first n seconds of a target audio",
    )
    args = parser.parse_args()
    offset = find_offset(args.within, args.find_offset_of, args.window)
    print(f"Offset: {offset}s")


if __name__ == "__main__":
    for within, offset_of in [
        (
            (AUDIO_SAMPLES / "muse_uprising.mp3").absolute(),
            (AUDIO_SAMPLES / "muse_preview.mp3").absolute(),
        ),
        # (
        #     (AUDIO_SAMPLES / "roddy.wav").absolute(),
        #     (AUDIO_SAMPLES / "muse_preview.mp3").absolute(),
        # ),
    ]:
        offset = find_offset(within, offset_of)
        print("offset", offset)
