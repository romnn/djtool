#!/bin/bash

set -o errexit

brew install git doxygen asciidoc wget cmake autoconf automake nasm libtool ninja meson pkg-config rtmpdump libtool

# THREADS=$(sysctl -n hw.ncpu)
THREADS=$(sysctl -n hw.ncpu)
TARGET=$(realpath "./ffmpeg-build")
CMPL="$TARGET/build"
mkdir -p "$CMPL"
# rm -fr $CMPL/*
export PATH="${TARGET}/bin:/usr/local/bin:/usr/bin:/bin:/usr/sbin:/sbin:/usr/local/include:/usr/local/opt:/usr/local/Cellar:/usr/local/lib:/usr/local/share:/usr/local/etc"

if false; then
  # xz
  cd ${CMPL}
  git -c http.sslVerify=false clone https://git.tukaani.org/xz.git
  cd xz
  ./autogen.sh || (echo "" > /dev/null)
  ./configure --prefix=${TARGET} --enable-static --disable-shared --disable-docs --disable-examples
  make -j "$THREADS" && make install
  rm -fr $CMPL/*


  # libexpat
  cd ${CMPL}
  git clone https://github.com/libexpat/libexpat.git libexpat
  cd libexpat/expat
  ./buildconf.sh
  ./configure --prefix=${TARGET} CPPFLAGS=-DXML_LARGE_SIZE --enable-static
  make -j "$THREADS" && make install DESTDIR=/
  rm -fr $CMPL/*

  # iconv
  cd ${CMPL}
  wget --no-check-certificate "https://ftp.gnu.org/pub/gnu/libiconv/libiconv-1.16.tar.gz"
  tar -zxvf libiconv*
  cd libiconv*/
  ./configure --prefix=${TARGET} --enable-static --enable-extra-encodings
  make -j "$THREADS" && make install
  rm -fr $CMPL/*

  # gettext - Requirement for fontconfig, fribidi
  cd ${CMPL}
  wget --no-check-certificate "https://ftp.gnu.org/pub/gnu/gettext/gettext-0.21.tar.gz"
  tar -zxvf gettex*
  cd gettex*/
  ./configure --prefix=${TARGET} --disable-dependency-tracking --disable-silent-rules --disable-debug --with-included-gettext --with-included-glib \
   --with-included-libcroco --with-included-libunistring --with-included-libxml --with-emacs --disable-java --disable-native-java --disable-csharp \
   --disable-shared --enable-static --without-git --without-cvs --disable-docs --disable-examples
  make -j "$THREADS" && make install
  rm -fr $CMPL/*

  # libpng git - Requirement for freetype
  cd ${CMPL}
  git clone https://github.com/glennrp/libpng.git
  cd libpng
  autoreconf -fiv
  ./configure --prefix=${TARGET} --disable-dependency-tracking --disable-silent-rules --enable-static --disable-shared
  make -j "$THREADS" && make install
  rm -fr $CMPL/*

  # pkg-config
  cd ${CMPL}
  LastVersion=$(wget --no-check-certificate 'https://pkg-config.freedesktop.org/releases/' -O- -q | grep -Eo 'pkg-config-0.29[0-9\.]+\.tar.gz' | tail -1)
  wget --no-check-certificate 'https://pkg-config.freedesktop.org/releases/'"$LastVersion"
  tar -zxvf pkg-config-*
  cd pkg-config-*/
  ./configure --prefix=${TARGET} --disable-debug --disable-host-tool --with-internal-glib
  make -j "$THREADS" && make check && make install
  rm -fr $CMPL/*

  # Yasm
  LastVersion=$(wget --no-check-certificate 'http://www.tortall.net/projects/yasm/releases/' -O- -q | grep -Eo 'yasm-[0-9\.]+\.tar.gz' | tail -1)
  cd ${CMPL}
  wget --no-check-certificate 'http://www.tortall.net/projects/yasm/releases/'"$LastVersion"
  tar -zxvf $CMPL/yasm-*
  cd yasm-*/
  ./configure --prefix=${TARGET} && make -j "$THREADS" && make install
  rm -fr $CMPL/*

  # bzip2
  cd ${CMPL}
  git clone git://sourceware.org/git/bzip2.git bzip2
  cd bzip2
  make -j "$THREADS" && make install PREFIX=${TARGET}
  rm -fr $CMPL/*
fi

  # SDL2
  # requires correct iconv
  # CPPFLAGS='-I/opt/local/include' LDFLAGS='-L/opt/local/lib'
  cd ${CMPL}
  wget http://www.libsdl.org/release/SDL2-2.0.14.tar.gz
  tar xvf SDL2-*.tar.gz
  cd SDL2*/
  ./autogen.sh
  ./configure --prefix=${TARGET} --enable-static --disable-shared --with-iconv=$TARGET/include --with-iconv-dir=$TARGET/include --with-libiconv-prefix=$TARGET/include --without-x --enable-hidapi
  make -j "$THREADS" && make install
  rm -fr $CMPL/*

if false; then
  # libudfread git
  cd ${CMPL}
  git clone https://github.com/vlc-mirror/libudfread.git
  cd libud*/
  ./bootstrap
  ./configure --prefix=${TARGET} --disable-shared --enable-static
  make -j "$THREADS" && make install

  #_ bluray git
  JAVAV=$(find /Library/Java/JavaVirtualMachines -iname "*.jdk" | tail -1)
  export JAVA_HOME="$JAVAV/Contents/Home"
  cd ${CMPL}
  git -c http.sslVerify=false clone https://code.videolan.org/videolan/libbluray.git
  cd libblura*/
  cp -r $CMPL/libudfread/src $CMPL/libbluray/contrib/libudfread/src
  ./bootstrap
  ./configure --prefix=${TARGET} --disable-shared --disable-dependency-tracking --build x86_64 --disable-doxygen-dot --without-libxml2 --without-freetype --disable-udf --disable-bdjava-jar
  cp -vpfr $CMPL/libblura*/jni/darwin/jni_md.h $CMPL/libblura*/jni
  make -j "$THREADS" && make install
  rm -fr $CMPL/*

  # freetype
  LastVersion=$(wget --no-check-certificate 'https://download.savannah.gnu.org/releases/freetype/' -O- -q | grep -Eo 'freetype-[0-9\.]+\.10+\.[0-9\.]+\.tar.gz' | tail -1)
  cd ${CMPL}
  wget --no-check-certificate 'https://download.savannah.gnu.org/releases/freetype/'"$LastVersion"
  tar xzpf freetype-*
  cd freetype-*/
  pip3 install docwriter
  ./configure --prefix=${TARGET} --disable-shared --enable-static
  make -j "$THREADS" && make install
  rm -fr $CMPL/*

  # fribidi
  cd ${CMPL}
  wget --no-check-certificate -O fribidi-1.0.10.tar.xz https://github.com/fribidi/fribidi/releases/download/v1.0.10/fribidi-1.0.10.tar.xz
  tar -xJf fribid*
  cd fribid*/
  ./configure --prefix=${TARGET} --disable-shared --enable-static --disable-silent-rules --disable-debug --disable-dependency-tracking
  make -j "$THREADS" && make install
  rm -fr $CMPL/*

  # fontconfig
  cd ${CMPL}
  wget --no-check-certificate https://www.freedesktop.org/software/fontconfig/release/fontconfig-2.13.92.tar.gz
  tar xzpf fontconfig-*
  cd fontconfig-*/
  ./configure --prefix=${TARGET} --disable-dependency-tracking --disable-silent-rules --with-add-fonts="/System/Library/Fonts,/Library/Fonts" --disable-shared --enable-static
  make -j "$THREADS" && make install
  rm -fr $CMPL/*

  # harfbuzz
  cd ${CMPL}
  git clone https://github.com/harfbuzz/harfbuzz.git
  cd harfbuzz
  ./autogen.sh
  ./configure --prefix=${TARGET} --disable-shared --enable-static
  make -j "$THREADS" && make install
  rm -fr $CMPL/*

  # libass
  # requires harfbuzz
  cd ${CMPL}
  git clone https://github.com/libass/libass.git
  cd libas*/
  ./autogen.sh
  ./configure --prefix=${TARGET} --disable-shared --enable-static
  make -j "$THREADS" && make install
  rm -fr $CMPL/*

  # openssl
  LastVersion=$(wget --no-check-certificate 'https://www.openssl.org/source/' -O- -q | grep -Eo 'openssl-[0-9\.]+\.[0-9\.]+\.[0-9\.]+[A-Za-z].tar.gz' | tail -1)
  cd ${CMPL}
  wget --no-check-certificate https://www.openssl.org/source/"$LastVersion"
  tar -zxvf openssl*
  cd openssl-*/
  ./Configure --prefix=${TARGET} -openssldir=${TARGET}/usr/local/etc/openssl no-ssl3 no-zlib enable-cms darwin64-x86_64-cc shared enable-ec_nistp_64_gcc_128
  make -j "$THREADS" depend && make install_sw
  rm -fr $CMPL/*

  # srt
  # requires openssl
  cd ${CMPL}
  git clone --depth 1 https://github.com/Haivision/srt.git
  cd srt/
  mkdir build && cd build
  cmake -G "Ninja" .. -DCMAKE_INSTALL_PREFIX:PATH=${TARGET} -DENABLE_C_DEPS=ON -DENABLE_SHARED=OFF -DENABLE_STATIC=ON
  ninja && ninja install
  rm -fr $CMPL/*

  # snappy
  cd ${CMPL}
  wget -O snappy.tar.gz --no-check-certificate https://github.com/google/snappy/archive/1.1.8.tar.gz
  tar -zxvf snappy.tar.gz
  cd snappy-*/
  mkdir build && cd build
  cmake -G "Ninja" ../ -DCMAKE_INSTALL_PREFIX:PATH=${TARGET} -DENABLE_SHARED="OFF" -DENABLE_C_DEPS="ON"
  ninja && ninja install
  rm -fr $CMPL/*

  # openal-soft
  cd ${CMPL}
  git clone https://github.com/kcat/openal-soft
  cd openal-soft*/
  cmake -G "Ninja" -DCMAKE_INSTALL_PREFIX:PATH=${TARGET} -DLIBTYPE=STATIC .
  ninja && ninja install
  rm -fr $CMPL/*

  # opencore-amr
  cd ${CMPL}
  wget -O opencore-amr-0.1.5.tar.gz http://freefr.dl.sourceforge.net/project/opencore-amr/opencore-amr/opencore-amr-0.1.5.tar.gz
  tar -zxvf opencore-amr-*
  cd opencore-amr-*
  ./configure --prefix=${TARGET} --disable-shared --enable-static
  make -j "$THREADS" && make install
  rm -fr $CMPL/*

  # opus - Replace speex
  LastVersion=$(wget --no-check-certificate https://ftp.osuosl.org/pub/xiph/releases/opus/ -O- -q | grep -Eo 'opus-1.[0-9\.]+\.[0-9\.]+\.tar.gz' | tail -1)
  cd ${CMPL}
  wget --no-check-certificate https://ftp.osuosl.org/pub/xiph/releases/opus/"$LastVersion"
  tar -zxvf opus-*
  cd opus-*/
  ./configure --prefix=${TARGET} --disable-shared --enable-static
  make -j "$THREADS" && make install
  rm -fr $CMPL/*

  # ogg
  LastVersion=$(wget --no-check-certificate https://ftp.osuosl.org/pub/xiph/releases/ogg/ -O- -q | grep -Eo 'libogg-[0-9\.]+\.tar.gz' | tail -1)
  cd ${CMPL}
  wget --no-check-certificate https://ftp.osuosl.org/pub/xiph/releases/ogg/"$LastVersion"
  tar -zxvf libogg-*
  cd libogg-*/
  ./configure --prefix=${TARGET} --disable-shared --enable-static --disable-dependency-tracking
  make -j "$THREADS" && make install
  rm -fr $CMPL/*

  # Theora
  # requires nf automake libtool
  cd ${CMPL}
  git clone https://github.com/xiph/theora.git
  cd theora
  ./autogen.sh
  ./configure --prefix=${TARGET} --with-ogg-libraries=${TARGET}/lib --with-ogg-includes=${TARGET}/include/ --with-vorbis-libraries=${TARGET}/lib --with-vorbis-includes=${TARGET}/include/ --enable-static --disable-shared
  make -j "$THREADS" && make install
  rm -fr $CMPL/*

  # vorbis
  LastVersion=$(wget --no-check-certificate https://ftp.osuosl.org/pub/xiph/releases/vorbis/ -O- -q | grep -Eo 'libvorbis-[0-9\.]+\.tar.gz' | tail -1)
  cd ${CMPL}
  wget --no-check-certificate https://ftp.osuosl.org/pub/xiph/releases/vorbis/"$LastVersion"
  tar -zxvf libvorbis-*
  cd libvorbis-*/
  ./configure --prefix=${TARGET} --with-ogg-libraries=${TARGET}/lib --with-ogg-includes=${TARGET}/include/ --enable-static --disable-shared
  make -j "$THREADS" && make install
  rm -fr $CMPL/*

  # lame
  cd ${CMPL}
  git clone https://github.com/rbrito/lame.git
  cd lam*/
  ./configure --prefix=${TARGET} --disable-shared --enable-static
  make -j "$THREADS" && make install
  rm -fr $CMPL/*

  # TwoLame - optimised MPEG Audio Layer 2
  cd ${CMPL}
  # LastVersion=$(wget --no-check-certificate 'http://www.twolame.org' -O- -q | grep -Eo 'twolame-[0-9\.]+\.tar.gz' | tail -1)
  # wget --no-check-certificate -O twolame.tar.gz 'http://downloads.sourceforge.net/twolame/'"$LastVersion"
  # tar -zxvf twolame.tar.gz
  git clone https://github.com/njh/twolame.git
  cd twolam*/
  ./autogen.sh
  ./configure --prefix=${TARGET} --enable-static --enable-shared=no
  make -j "$THREADS" && make install
  rm -fr $CMPL/*

  # fdk-aac
  cd ${CMPL}
  wget -O fdk-aac-2.0.1.tar.gz http://freefr.dl.sourceforge.net/project/opencore-amr/fdk-aac/fdk-aac-2.0.1.tar.gz
  tar -zxvf fdk-aac-*
  cd fdk*/
  ./configure --disable-dependency-tracking --prefix=${TARGET} --enable-static --enable-shared=no
  make -j "$THREADS" && make install
  rm -fr $CMPL/*

  # gsm
  cd ${CMPL}
  wget --no-check-certificate 'http://www.quut.com/gsm/gsm-1.0.19.tar.gz'
  tar -zxvf gsm*
  cd gsm*/
  mkdir -p ${TARGET}/man/man3
  mkdir -p ${TARGET}/man/man1
  mkdir -p ${TARGET}/include/gsm
  perl -p -i -e "s#^INSTALL_ROOT.*#INSTALL_ROOT = $TARGET#g" Makefile
  perl -p -i -e "s#_ROOT\)/inc#_ROOT\)/include#g" Makefile
  sed "/GSM_INSTALL_INC/s/include/include\/gsm/g" Makefile > Makefile.new
  mv Makefile.new Makefile
  make -j "$THREADS" && make install
  rm -fr $CMPL/*

  # speex
  cd ${CMPL}
  wget http://downloads.us.xiph.org/releases/speex/speex-1.2.0.tar.gz
  tar xvf speex-1.2.0.tar.gz
  cd speex-1.2.0
  ./configure --prefix=${TARGET} --enable-static --enable-shared=no
  make -j "$THREADS" && make install
  rm -fr $CMPL/*

  # libzimg
  cd ${CMPL}
  git clone https://github.com/sekrit-twc/zimg.git
  cd zimg
  ./autogen.sh
  ./Configure --prefix=${TARGET} --disable-shared --enable-static
  make -j "$THREADS" && make install
  rm -fr $CMPL/*

  # libvpx
  cd ${CMPL}
  git clone https://github.com/webmproject/libvpx.git
  cd libvp*/
  ./configure --prefix=${TARGET} --enable-vp8 --enable-postproc --enable-vp9-postproc --enable-vp9-highbitdepth --disable-examples --disable-docs --enable-multi-res-encoding --disable-unit-tests --enable-pic --disable-shared
  make -j "$THREADS" && make install
  rm -fr $CMPL/*

  # webp
  cd ${CMPL}
  git clone https://chromium.googlesource.com/webm/libwebp
  cd libweb*/
  ./autogen.sh
  ./configure --prefix=${TARGET} --disable-dependency-tracking --disable-gif --disable-gl --enable-libwebpdecoder --enable-libwebpdemux --enable-libwebpmux
  make -j "$THREADS" && make install
  rm -fr $CMPL/*

  # openjpeg
  cd ${CMPL}
  git clone https://github.com/uclouvain/openjpeg.git
  cd openjpeg
  mkdir build && cd build
  cmake -G "Ninja" .. -DCMAKE_INSTALL_PREFIX:PATH=${TARGET} -DLIBTYPE=STATIC
  ninja && ninja install
  rm -fr $CMPL/*

  # av1
  cd ${CMPL}
  git clone https://aomedia.googlesource.com/aom
  cd aom
  mkdir aom_build && cd aom_build
  cmake -G "Ninja" $CMPL/aom -DCMAKE_INSTALL_PREFIX:PATH=${TARGET} -DLIBTYPE=STATIC
  ninja && ninja install
  rm -fr $CMPL/*

  # dav1d git - Require ninja, meson
  cd ${CMPL}
  git -c http.sslVerify=false clone https://code.videolan.org/videolan/dav1d.git
  cd dav1*/
  meson --prefix=${TARGET} build --buildtype release --default-library static
  ninja install -C build
  rm -fr $CMPL/*

  # xvid
  LastVersion=$(wget --no-check-certificate https://downloads.xvid.com/downloads/ -O- -q | grep -Eo 'xvidcore-[0-9\.]+\.tar.gz' | tail -1)
  cd ${CMPL}
  wget --no-check-certificate https://downloads.xvid.com/downloads/"$LastVersion"
  tar -zxvf xvidcore*
  cd xvidcore/build/generic/
  ./bootstrap.sh
  ./configure --prefix=${TARGET} --disable-assembly --enable-macosx_module
  make -j "$THREADS" && make install
  rm -fr $CMPL/*

  # openh264
  cd ${CMPL}
  git clone https://github.com/cisco/openh264.git
  cd openh264/
  make -j "$THREADS" install-static PREFIX=${TARGET}
  rm -fr $CMPL/*

  # x264 8-10bit git - Require nasm
  cd ${CMPL}
  git -c http.sslVerify=false clone https://code.videolan.org/videolan/x264.git
  cd x264/
  ./configure --prefix=${TARGET} --enable-static --bit-depth=all --chroma-format=all --enable-mp4-output
  make -j "$THREADS" && make install
  rm -fr $CMPL/*

  # x265 8-10-12bit - Require wget, cmake, yasm, nasm, libtool, ninja
  cd ${CMPL}
  git clone https://bitbucket.org/multicoreware/x265_git/src/master/ x265-master
  cd x265*/source/
  mkdir -p 8bit 10bit 12bit

  cd 12bit
  cmake -G "Ninja" ../../../x265*/source -DCMAKE_INSTALL_PREFIX:PATH=${TARGET} -DHIGH_BIT_DEPTH=ON -DEXPORT_C_API=OFF -DENABLE_SHARED=OFF -DENABLE_CLI=OFF -DMAIN12=ON
  ninja ${MAKEFLAGS}

  cd ../10bit
  cmake -G "Ninja" ../../../x265*/source -DCMAKE_INSTALL_PREFIX:PATH=${TARGET} -DHIGH_BIT_DEPTH=ON -DEXPORT_C_API=OFF -DENABLE_SHARED=OFF -DENABLE_CLI=OFF
  ninja ${MAKEFLAGS}

  cd ../8bit
  ln -sf ../10bit/libx265.a libx265_main10.a
  ln -sf ../12bit/libx265.a libx265_main12.a

  cmake -G "Ninja" ../../../x265*/source -DCMAKE_INSTALL_PREFIX:PATH=${TARGET} -DENABLE_SHARED=NO -DEXTRA_LIB="x265_main10.a;x265_main12.a" -DEXTRA_LINK_FLAGS=-L. -DLINKED_10BIT=ON -DLINKED_12BIT=ON
  ninja ${MAKEFLAGS}

  #_ rename the 8bit library, then combine all three into libx265.a
  mv libx265.a libx265_main.a
  libtool -static -o libx265.a libx265_main.a libx265_main10.a libx265_main12.a
  ninja install
  rm -fr $CMPL/*

  #_ AviSynth+
  cd ${CMPL}
  git clone https://github.com/AviSynth/AviSynthPlus.git
  cd AviSynthPlus
  mkdir avisynth-build && cd avisynth-build
  cmake ../ -DCMAKE_INSTALL_PREFIX:PATH=${TARGET} -DHEADERS_ONLY:bool=on
  make install
  rm -fr $CMPL/*
fi

# librtmp
# cp -v /usr/local/Cellar/rtmpdump/2.4+20151223_1/bin/* /Volumes/RamDisk/sw/bin/
# cp -vr /usr/local/Cellar/rtmpdump/2.4+20151223_1/include/* /Volumes/RamDisk/sw/include/
# cp -v /usr/local/Cellar/rtmpdump/2.4+20151223_1/lib/pkgconfig/librtmp.pc /Volumes/RamDisk/sw/lib/pkgconfig
# cp -v /usr/local/Cellar/rtmpdump/2.4+20151223_1/lib/librtmp* /Volumes/RamDisk/sw/lib

export LDFLAGS="-L${TARGET}/lib -Wl,-framework,OpenAL"
export CPPFLAGS="-I${TARGET}/include -Wl,-framework,OpenAL"
export CFLAGS="-I${TARGET}/include -Wl,-framework,OpenAL,-fno-stack-check"

# Minimal ffmpeg
cd ${CMPL}
rm -fr ${CMPL}/ffmpeg
git clone git://git.ffmpeg.org/ffmpeg.git
cd ffmpe*/
./configure \
  --prefix=${TARGET} \
  --pkg_config='pkg-config --static' \
  --extra-cflags="-fno-stack-check" \
  --cc=/usr/bin/clang \
  --disable-everything \
  --disable-network \
  --disable-autodetect \
  --enable-pthreads \
  --enable-small \
  --enable-nonfree \
  --enable-libfdk-aac \
  --enable-decoder=aac*,ac3*,opus,vorbis \
  --enable-demuxer=mov,m4v,matroska \
  --enable-muxer=mp3,mp4 \
  --enable-protocol=file \
  --enable-libmp3lame \
  --enable-encoder=aac \
  --enable-filter=aresample

make -j "$THREADS" && make install
otool -L $TARGET/bin/ffmpeg
exit 0

# FFmpeg
cd ${CMPL}
rm -fr ${CMPL}/ffmpeg
git clone git://git.ffmpeg.org/ffmpeg.git
cd ffmpe*/
./configure --extra-version=adam-"$(date +"%Y-%m-%d")" --extra-cflags="-fno-stack-check" --arch=x86_64 --cc=/usr/bin/clang \
 --enable-pthreads --enable-postproc --enable-runtime-cpudetect \
 --pkg_config='pkg-config --static' --enable-nonfree --enable-gpl --enable-version3 --prefix=${TARGET} \
 --disable-ffplay --disable-ffprobe --disable-debug --disable-doc --enable-avfilter --enable-avisynth --enable-filters \
 --enable-libopus --enable-libvorbis --enable-libtheora --enable-libspeex --enable-libmp3lame --enable-libfdk-aac --enable-encoder=aac \
 --enable-libopencore_amrwb --enable-libopencore_amrnb --enable-libopencore_amrwb --enable-libgsm \
 --enable-muxer=mp4 --enable-libxvid --enable-libopenh264 --enable-libx264 --enable-libx265 --enable-libvpx --enable-libaom --enable-libdav1d \
 --enable-fontconfig --enable-libfreetype --enable-libfribidi --enable-libass --enable-libsrt \
 --enable-libbluray --enable-bzlib --enable-zlib --enable-lzma --enable-libsnappy --enable-libwebp --enable-libopenjpeg \
 --enable-opengl --enable-opencl --enable-openal --enable-libzimg --enable-openssl 

# --enable-libtwolame
# --enable-librtmp
make -j "$THREADS" && make install

# Check Static
otool -L $TARGET/bin/ffmpeg
if otool -L $TARGET/bin/ffmpeg | grep /usr/local
then echo FFmpeg build Not Static, Please Report
else echo FFmpeg build Static, Have Fun
fi
