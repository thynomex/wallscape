#!/usr/bin/env bash
# Run from the isolated MSYS2 UCRT64 environment prepared by build-libmpv-slim.ps1.
set -euo pipefail

root=$(cygpath -u "${1:?build workspace required}")
stage=${2:-all}
jobs=${WALLSCAPE_BUILD_JOBS:-8}
prefix="$root/native-prefix"
sources="$root/sources"
deps="$root/native-deps"
mkdir -p "$prefix/lib/pkgconfig" "$root/native-build" "$root/logs"
export PATH="/ucrt64/bin:/usr/bin:$PATH"
export CC=gcc CXX=g++ AR=ar RANLIB=ranlib
export PKG_CONFIG_PATH="$prefix/lib/pkgconfig:/ucrt64/lib/pkgconfig:/ucrt64/share/pkgconfig"
export PKG_CONFIG_LIBDIR="$PKG_CONFIG_PATH"
export CFLAGS='-O2 -march=x86-64 -ffunction-sections -fdata-sections'
export CXXFLAGS="$CFLAGS"
export LDFLAGS='-static -static-libgcc -static-libstdc++ -Wl,--gc-sections'
# mpv embeds its Meson options in the DLL, so a timestamp linker flag would
# change the code layout. binutils stamps PE headers from SOURCE_DATE_EPOCH
# instead; use the pinned mpv commit time so rebuilds produce identical bytes.
if [ -z "${SOURCE_DATE_EPOCH:-}" ] && git -C "$sources/mpv" rev-parse HEAD > /dev/null 2>&1; then
    export SOURCE_DATE_EPOCH=$(git -C "$sources/mpv" log -1 --format=%ct)
fi
# Meson's CMake dependency fallback aborts without a resolvable home directory
# for the native Python. Keep it inside the isolated workspace.
if [ -z "${USERPROFILE:-}" ]; then
    export USERPROFILE=$(cygpath -w "$root/home")
fi
mkdir -p "$USERPROFILE" "$root/home"

configure_meson() {
    local mode=$1
    local dir=$2
    shift 2
    if [ ! -f "$dir/meson-private/coredata.dat" ]; then
        meson setup "$dir" "$@"
    elif [ "$mode" = wipe ]; then
        # Meson caches pkg-config results. The generated .pc files above decide
        # the linked libraries, so drop the cache instead of reusing it.
        meson setup --wipe "$dir" "$@"
    else
        meson setup --reconfigure "$dir" "$@"
    fi
}

# Package names requested by mpv describe shared libraries. Supply their real
# static equivalents, including all transitive shader/compiler libraries.
prepare_static_metadata() {
    local native_ucrt
    native_ucrt=$(cygpath -m /ucrt64)
    local shaderc_version
    shaderc_version=$(pkgconf --modversion shaderc_combined)
    # Meson's shaderc factory tries the static pkg-config names
    # (shaderc_combined, shaderc_static) before the plain `shaderc` name, so all
    # three must resolve to the static archives. The upstream files list only
    # -lshaderc_combined; the SPIRV-Tools and glslang symbols it references must
    # be linked explicitly.
    local shaderc_libs="-L$native_ucrt/lib -Wl,--start-group -lshaderc_combined -lSPIRV-Tools-opt -lSPIRV-Tools -lSPIRV-Tools-link -lSPIRV -lglslang -lMachineIndependent -lGenericCodeGen -lOSDependent -Wl,--end-group -lstdc++ -lpthread"
    for name in shaderc shaderc_combined shaderc_static; do
        cat > "$prefix/lib/pkgconfig/$name.pc" <<EOF
Name: shaderc
Description: MSYS2 shaderc combined static archive
Version: $shaderc_version
Libs: $shaderc_libs
Cflags: -I$native_ucrt/include
EOF
    done
    cat > "$prefix/lib/pkgconfig/spirv-cross-c-shared.pc" <<EOF
Name: spirv-cross-c-shared
Description: Static SPIRV-Cross C API and backends
Version: $(pkgconf --modversion spirv-cross-c)
Libs: -L$native_ucrt/lib -Wl,--start-group -lspirv-cross-c -lspirv-cross-glsl -lspirv-cross-hlsl -lspirv-cross-msl -lspirv-cross-cpp -lspirv-cross-reflect -lspirv-cross-util -lspirv-cross-core -Wl,--end-group -lstdc++
Cflags: -I$native_ucrt/include/spirv_cross
EOF
    # Meson prefers static dependencies; FFmpeg receives --static explicitly.
    # Use the native executable so Windows Python never resolves a WSL bash.
    export PKG_CONFIG=pkgconf
}

build_dependencies() {
    prepare_static_metadata
    configure_meson reconfigure "$root/native-build/libplacebo" "$deps/libplacebo" \
        --prefix="$(cygpath -m "$prefix")" --libdir=lib --buildtype=release \
        --default-library=static -Dprefer_static=true -Dauto_features=disabled \
        -Dd3d11=enabled -Dshaderc=enabled -Dglslang=disabled \
        -Dvulkan=disabled -Dopengl=disabled -Dlcms=disabled \
        -Ddovi=disabled -Dlibdovi=disabled -Dunwind=disabled -Dxxhash=disabled \
        -Ddemos=false -Dtests=false -Dbench=false -Dfuzz=false \
        -Dc_link_args="$LDFLAGS" -Dcpp_link_args="$LDFLAGS"
    meson compile -C "$root/native-build/libplacebo" -j "$jobs"
    meson install -C "$root/native-build/libplacebo"
}

build_ffmpeg() {
    prepare_static_metadata
    mkdir -p "$root/native-build/ffmpeg"
    cd "$root/native-build/ffmpeg"
    # version.sh prefers an inherited revision. Pin it so the exported
    # ffmpeg-version string does not depend on whether git was installed or on
    # how the source checkout was made. FFmpeg's own message for a checkout
    # without tags is "git-YYYY-MM-DD-hhhhhhh"; match that.
    if git -C "$sources/ffmpeg" rev-parse HEAD > /dev/null 2>&1; then
        export revision="git-$(git -C "$sources/ffmpeg" log -1 --pretty=format:%cd-%h --date=short)"
    fi
    # Start from nothing and add back exactly what Wallscape plays. The app
    # accepts local MP4/MOV/M4V, WebM/Matroska, AVI, and WMV, and the validation
    # suite also covers FLV, MPEG-TS, GIF, and raw YUV. Audio output, encoding,
    # networking, capture devices, disc/archive input, and unrelated external
    # libraries stay out. Enablement is name-based, so each list is explicit.
    local demuxers=mov,matroska,avi,asf,flv,mpegts,mpegps,mpegvideo,gif,yuv4mpegpipe,rawvideo,image2
    local decoders=h264,hevc,vp8,vp9,av1,libdav1d,mpeg4,mpeg2video,mpeg1video,msmpeg4v2,msmpeg4v3,wmv1,wmv2,wmv3,vc1,mjpeg,flv,gif,png,rawvideo,h263,prores,dnxhd,ffv1,huffyuv,utvideo,qtrle,msrle
    local parsers=h264,hevc,vp8,vp9,av1,mpeg4video,mpegvideo,mjpeg,png,vc1,aac,opus,vorbis
    # *_d3d11va copies frames back to system memory; *_d3d11va2 keeps them as
    # GPU surfaces, which is what mpv uses for hwdec=d3d11va. Without the "2"
    # variants mpv silently falls back to dxva2-copy.
    local hwaccels=h264_d3d11va,h264_d3d11va2,h264_dxva2,hevc_d3d11va,hevc_d3d11va2,hevc_dxva2,vp9_d3d11va,vp9_d3d11va2,vp9_dxva2,av1_d3d11va,av1_d3d11va2,av1_dxva2,mpeg2_d3d11va,mpeg2_d3d11va2,mpeg2_dxva2,vc1_d3d11va,vc1_d3d11va2,vc1_dxva2,wmv3_d3d11va,wmv3_d3d11va2,wmv3_dxva2
    local filters=select,metadata,showinfo,format,scale,null,hwdownload,hwupload
    local enables=()
    local name
    for name in ${demuxers//,/ }; do enables+=(--enable-demuxer="$name"); done
    for name in ${decoders//,/ }; do enables+=(--enable-decoder="$name"); done
    for name in ${parsers//,/ }; do enables+=(--enable-parser="$name"); done
    for name in ${hwaccels//,/ }; do enables+=(--enable-hwaccel="$name"); done
    for name in ${filters//,/ }; do enables+=(--enable-filter="$name"); done
    "$sources/ffmpeg/configure" \
        --prefix="$(cygpath -m "$prefix")" --arch=x86_64 --target-os=mingw32 \
        --cc=gcc --cxx=g++ --ar=ar --ranlib=ranlib --nm=nm --strip=strip \
        --disable-autodetect --disable-shared --enable-static \
        --disable-debug --disable-doc --disable-programs --disable-network \
        --disable-devices --disable-everything \
        --enable-protocol=file --enable-bsfs \
        --enable-zlib --enable-libdav1d --enable-d3d11va --enable-dxva2 \
        --disable-vdpau --disable-videotoolbox \
        "${enables[@]}" \
        --pkg-config="$PKG_CONFIG" --pkg-config-flags=--static \
        --extra-cflags="$CFLAGS" --extra-ldflags="$LDFLAGS" --extra-libs='-lstdc++'
    # Refuse a superficially successful configuration that omitted cap/test/VO dependencies.
    for required in SELECT_FILTER SCENE_SAD METADATA_FILTER SHOWINFO_FILTER \
        FORMAT_FILTER SCALE_FILTER YUV4MPEGPIPE_DEMUXER RAWVIDEO_DECODER \
        H264_DECODER HEVC_DECODER VP9_DECODER LIBDAV1D_DECODER H264_D3D11VA_HWACCEL; do
        grep -q "#define CONFIG_${required} 1" config_components.h config.h || {
            echo "Missing required FFmpeg component: $required" >&2; exit 1;
        }
    done
    make -j "$jobs"
    make install
}

build_mpv() {
    prepare_static_metadata
    # Reconfigure every time: the shaderc/spirv-cross metadata above changes the
    # resolved link arguments, and a cached configure keeps the stale ones.
    configure_meson wipe "$root/native-build/mpv" "$sources/mpv" \
        --prefix="$(cygpath -m "$prefix")" --libdir=lib --buildtype=release \
        --default-library=shared -Dprefer_static=true -Dauto_features=disabled \
        -Db_lto=true -Db_lto_threads="$jobs" -Db_ndebug=true \
        -Dlibmpv=true -Dcplayer=false -Dbuild-date=false \
        -Dd3d11=enabled -Dd3d-hwaccel=enabled -Dshaderc=enabled \
        -Dspirv-cross=enabled -Dzlib=enabled -Dwin32-threads=enabled \
        -Dgl=disabled -Dvulkan=disabled -Dwasapi=disabled \
        -Dc_args="$CFLAGS" -Dc_link_args="$LDFLAGS -lstdc++"
    # Meson resolves target names, not output filenames; the library is `mpv`.
    meson compile -C "$root/native-build/mpv" -j "$jobs" mpv
    local package="$root/candidate/mpv-dev-slim-x86_64-c152964208"
    mkdir -p "$package/include/mpv" "$package/licenses" "$package/provenance"
    cp "$root/native-build/mpv/libmpv-2.dll" "$package/"
    strip --strip-unneeded "$package/libmpv-2.dll"
    cp "$root/native-build/mpv/libmpv.dll.a" "$package/"
    # GNU import members are AMD64 COFF, accepted by MSVC LINK as mpv.lib.
    cp "$package/libmpv.dll.a" "$package/mpv.lib"
    cp "$sources/mpv/include/mpv/"*.h "$package/include/mpv/"
    cp "$sources/mpv/"LICENSE* "$sources/mpv/Copyright" "$package/licenses/"
    cp "$sources/ffmpeg/COPYING.LGPLv2.1" "$package/licenses/FFmpeg-LGPLv2.1.txt"
    cp "$deps/libplacebo/LICENSE" "$package/licenses/libplacebo.txt"
    cp "$root/native-build/ffmpeg/config.h" "$root/native-build/ffmpeg/config_components.h" \
       "$root/native-build/ffmpeg/ffbuild/config.mak" "$package/provenance/"
    cp "$root/native-build/mpv/meson-info/intro-buildoptions.json" "$package/provenance/mpv-options.json"
    cp "$root/native-build/libplacebo/meson-info/intro-buildoptions.json" "$package/provenance/libplacebo-options.json"
    pacman -Q > "$package/provenance/msys2-packages.txt"
    gcc --version > "$package/provenance/gcc-version.txt"
    write_provenance "$package"
    package_dll_archive "$package"
    echo "Candidate package: $(cygpath -m "$package")"
}

write_provenance() {
    local package=$1
    local mpv_rev ffmpeg_rev placebo_rev
    mpv_rev=$(git -C "$sources/mpv" rev-parse HEAD)
    ffmpeg_rev=$(git -C "$sources/ffmpeg" rev-parse HEAD)
    placebo_rev=$(git -C "$deps/libplacebo" rev-parse HEAD)
    cat > "$package/provenance/SOURCES.md" <<EOF
# Build sources

Built $(date -u +%Y-%m-%d) UTC with the MSYS2 UCRT64 toolchain recorded in
\`gcc-version.txt\` and \`msys2-packages.txt\`. Reproduce with
\`scripts/build-libmpv-slim.ps1\`.

| Component | Revision | Origin |
| --- | --- | --- |
| mpv | \`$mpv_rev\` | https://github.com/mpv-player/mpv.git |
| FFmpeg | \`$ffmpeg_rev\` | https://github.com/FFmpeg/FFmpeg.git |
| libplacebo | \`$placebo_rev\` | https://github.com/haasn/libplacebo.git |

mpv and FFmpeg match the revisions of upstream package
\`mpv-dev-x86_64-20261005-git-c152964208.7z\`, so the only differences from that
runtime are the enabled feature sets. \`config.h\`, \`config_components.h\`, and
\`config.mak\` record the FFmpeg configure result; \`mpv-options.json\` and
\`libplacebo-options.json\` record every Meson option.

## Licensing

mpv is GPLv2+ (\`LICENSE.GPL\`) with parts under LGPL (\`LICENSE.LGPL\`). FFmpeg is
configured without \`--enable-gpl\`; its terms are in \`FFmpeg-LGPLv2.1.txt\`.
libplacebo is LGPLv2.1+ (\`libplacebo.txt\`). The archive statically links these
libraries, so the distributed DLL carries the GPL.
EOF
}

package_dll_archive() {
    local package=$1
    local name
    name=$(basename "$package")
    ( cd "$package" && sha256sum libmpv-2.dll | cut -d' ' -f1 > libmpv-2.dll.sha256 )
    local archive="$root/candidate/$name.zip"
    ( cd "$root/candidate" && rm -f "$archive" && zip -q -r -9 "$archive" "$name" )
    echo "Archive: $(cygpath -m "$archive")"
    ( cd "$root/candidate" && sha256sum "$name.zip" > "$name.zip.sha256" )
}

case "$stage" in
    dependencies) build_dependencies ;;
    ffmpeg) build_ffmpeg ;;
    mpv) build_mpv ;;
    all) build_dependencies; build_ffmpeg; build_mpv ;;
    *) echo "Unknown build stage: $stage" >&2; exit 2 ;;
esac
