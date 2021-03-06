#!/bin/sh
# Based on https://github.com/zimbatm/ffmpeg-static

set -e
set -u

cd $(dirname "$0")
ENV_ROOT=$(pwd)
BUILD_DIR="$ENV_ROOT/native"
TARGET_DIR="$ENV_ROOT/target/native"
BIN_DIR="$TARGET_DIR/bin"
JVAL=5

export LDFLAGS="-L${TARGET_DIR}/lib"
export DYLD_LIBRARY_PATH="${TARGET_DIR}/lib"
export PKG_CONFIG_PATH="$TARGET_DIR/lib/pkgconfig"
# export CFLAGS="-I${TARGET_DIR}/include $LDFLAGS -static-libgcc -Wl,-Bstatic -lc"
# export CFLAGS="-I${TARGET_DIR}/include $LDFLAGS -Wl,-Bstatic -lc"
export CFLAGS="-I${TARGET_DIR}/include $LDFLAGS"
export PATH="${TARGET_DIR}/bin:${PATH}"
# Force PATH cache clearing
hash -r

mkdir -p "$TARGET_DIR" "$BIN_DIR"

echo "#### FFmpeg static build ####"

echo "*** Building x264 ***"
mkdir -p "$BUILD_DIR/x264"
cd $BUILD_DIR/x264
PATH="$BIN_DIR:$PATH" ./configure \
    --prefix=$TARGET_DIR \
    --cc="/usr/bin/clang" \
    --enable-static \
    --disable-opencl \
    --enable-pic \
    --disable-asm
PATH="$BIN_DIR:$PATH" make -j $JVAL
make install

# echo "*** Building mp3lame ***"
# mkdir -p "$BUILD_DIR/lame*"
# cd $BUILD_DIR/lame*
# ./configure --prefix=$TARGET_DIR \
#     --enable-nasm \
#     --disable-shared \
#     --with-pic=yes
# make -j $JVAL
# make install

echo "*** Building FFmpeg ***"
mkdir -p "$BUILD_DIR/ffmpeg"
cd $BUILD_DIR/ffmpeg
PATH="$BIN_DIR:$PATH" \
PKG_CONFIG_PATH="$TARGET_DIR/lib/pkgconfig" ./configure \
  --prefix="$TARGET_DIR" \
  --pkg-config-flags="--static" \
  --extra-cflags="-I$TARGET_DIR/include" \
  --extra-ldflags="-L$TARGET_DIR/lib" \
  --arch="x86_64" \
  --cc="/usr/bin/clang" \
  --bindir="$BIN_DIR" \
  --disable-doc  \
  --enable-gpl \
  --enable-version3 \
  --enable-nonfree \
  --enable-pic \
  --disable-asm \
  --disable-d3d11va \
  --disable-dxva2 \
  --disable-vaapi \
  --disable-vdpau \
  --disable-devices \
  --disable-bzlib \
  --disable-lzma \
  --disable-zlib \
  --enable-libmp3lame \
  --enable-libx264

# --disable-programs \
# --disable-vda \
# --enable-avresample \
# --pkg_config='pkg-config --static'
PATH="$BIN_DIR:$PATH" make -j $JVAL
make install
make distclean
hash -r
