#include "internal.h"
#include "video_wrapper.h"
#include <libavutil/avutil.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdint.h>

static int tests_passed = 0;
static int tests_failed = 0;

#define CHECK(cond, msg) do { \
    if (cond) { printf("  [PASS] %s\n", msg); tests_passed++; } \
    else      { printf("  [FAIL] %s\n", msg); tests_failed++; } \
} while (0)

// ---------------------------------------------------------------------
static void test_open_and_metadata(const char* path) {
    printf("\n== Test 1: open + metadata (%s) ==\n", path);

    VideoDemuxer* demuxer = demuxer_create(path);
    CHECK(demuxer != NULL, "demuxer_create succeeds on valid file");
    if (!demuxer) return;

    int w = demuxer_get_width(demuxer);
    int h = demuxer_get_height(demuxer);
    const char* codec = demuxer_get_codec_name(demuxer);

    printf("  width=%d height=%d codec=%s\n", w, h, codec);
    CHECK(w > 0, "width is positive");
    CHECK(h > 0, "height is positive");
    CHECK(codec != NULL && strcmp(codec, "unknown_codec") != 0, "codec name resolved");

    demuxer_free(demuxer);
}

// ---------------------------------------------------------------------
static int decode_full(const char* path, int* out_count) {
    VideoDemuxer* demuxer = demuxer_create(path);
    if (!demuxer) return -1;

    VideoDecoder* decoder = decoder_create(demuxer_get_codec_name(demuxer));
    if (!decoder) { demuxer_free(demuxer); return -1; }

    if (decoder_initialize_from_demuxer(decoder, demuxer) < 0) {
        decoder_free(decoder);
        demuxer_free(demuxer);
        return -1;
    }

    RawFrame* frame = frame_create();
    if (!frame) {
        decoder_free(decoder);
        demuxer_free(demuxer);
        return -1;
    }

    uint8_t* data; int size; int64_t pts;
    int frame_count = 0;
    int ret;
    int had_error = 0;

    while (demuxer_read_packet(demuxer, &data, &size, &pts) == 0) {
        if (decoder_send_packet(decoder, data, size, pts) < 0) {
            had_error = 1;
            break;
        }
        while ((ret = decoder_receive_frame(decoder, frame)) == 0) {
            frame_count++;
        }
        if (ret != AVERROR(EAGAIN) && ret != AVERROR_EOF) {
            printf("  unexpected receive_frame error: %d\n", ret);
            had_error = 1;
        }
    }

    decoder_send_packet(decoder, NULL, 0, 0);
    while ((ret = decoder_receive_frame(decoder, frame)) == 0) {
        frame_count++;
    }

    frame_free(frame);
    decoder_free(decoder);
    demuxer_free(demuxer);

    *out_count = frame_count;
    return had_error ? -1 : 0;
}

static void test_full_decode(const char* path, int expected_frames) {
    printf("\n== Test 2: full decode + flush (%s) ==\n", path);
    int count = 0;
    int ret = decode_full(path, &count);
    printf("  decoded %d frames (expected %d)\n", count, expected_frames);
    CHECK(ret == 0, "decode loop completed without error");
    if (expected_frames > 0) {
        CHECK(count == expected_frames, "frame count matches ffprobe -count_frames");
    } else {
        printf("  (no expected count given — run ffprobe manually to compare)\n");
    }
}

// ---------------------------------------------------------------------
static void test_repeat_decode(const char* path) {
    printf("\n== Test 3: repeat decode, same file twice ==\n");
    int count1 = 0, count2 = 0;
    int ret1 = decode_full(path, &count1);
    int ret2 = decode_full(path, &count2);
    printf("  run 1: %d frames, run 2: %d frames\n", count1, count2);
    CHECK(ret1 == 0 && ret2 == 0, "both runs completed without error");
    CHECK(count1 == count2, "frame counts match across independent runs");
}

// ---------------------------------------------------------------------
static void test_invalid_inputs(void) {
    printf("\n== Test 4: invalid input handling ==\n");

    VideoDemuxer* d1 = demuxer_create("this_file_does_not_exist.mp4");
    CHECK(d1 == NULL, "demuxer_create returns NULL for nonexistent file");
    if (d1) demuxer_free(d1);

    VideoDecoder* d2 = decoder_create("not_a_real_codec_name");
    CHECK(d2 == NULL, "decoder_create returns NULL for bogus codec name");
    if (d2) decoder_free(d2);

    demuxer_free(NULL);
    decoder_free(NULL);
    frame_free(NULL);
    printf("  [PASS] free functions tolerate NULL without crashing (if we got here)\n");
    tests_passed++;

    int ret = decoder_initialize_from_demuxer(NULL, NULL);
    CHECK(ret < 0, "decoder_initialize_from_demuxer rejects NULL args");
}

// ---------------------------------------------------------------------
static void test_frame_data_sanity(const char* path) {
    printf("\n== Test 5: frame data sanity (first frame) ==\n");

    VideoDemuxer* demuxer = demuxer_create(path);
    if (!demuxer) { printf("  [FAIL] could not open file\n"); tests_failed++; return; }

    VideoDecoder* decoder = decoder_create(demuxer_get_codec_name(demuxer));
    if (!decoder || decoder_initialize_from_demuxer(decoder, demuxer) < 0) {
        printf("  [FAIL] could not init decoder\n"); tests_failed++;
        decoder_free(decoder); demuxer_free(demuxer); return;
    }

    RawFrame* frame = frame_create();
    uint8_t* data; int size; int64_t pts;
    int got_frame = 0;

    while (!got_frame && demuxer_read_packet(demuxer, &data, &size, &pts) == 0) {
        if (decoder_send_packet(decoder, data, size, pts) < 0) break;
        if (decoder_receive_frame(decoder, frame) == 0) {
            got_frame = 1;
        }
    }

    CHECK(got_frame, "at least one frame decoded");
    if (got_frame) {
        uint8_t* y_plane = frame_get_data(frame, 0);
        int y_linesize = frame_get_linesize(frame, 0);
        int width = demuxer_get_width(demuxer);
        printf("  y_plane=%p linesize=%d width=%d\n",
            (void*)y_plane, y_linesize, width);
        CHECK(y_plane != NULL, "Y plane pointer is non-null");
        CHECK(y_linesize >= width, "linesize is at least frame width (padding is fine)");
        CHECK(frame_get_data(frame, 9) == NULL, "out-of-range plane index returns NULL, not garbage");
    }

    frame_free(frame);
    decoder_free(decoder);
    demuxer_free(demuxer);
}

// ---------------------------------------------------------------------
// Test 6: decode -> encode -> mux
// ---------------------------------------------------------------------
static void test_transcode_and_mux(const char* input_path, const char* output_path) {
    printf("\n== Test 6: decode -> encode -> mux (%s -> %s) ==\n", input_path, output_path);

    VideoDemuxer* demuxer = demuxer_create(input_path);
    if (!demuxer) { printf("  [FAIL] open input\n"); tests_failed++; return; }

    VideoDecoder* decoder = decoder_create(demuxer_get_codec_name(demuxer));
    if (!decoder || decoder_initialize_from_demuxer(decoder, demuxer) < 0) {
        printf("  [FAIL] init decoder\n"); tests_failed++;
        decoder_free(decoder); demuxer_free(demuxer); return;
    }

    int width = demuxer_get_width(demuxer);
    int height = demuxer_get_height(demuxer);
    AVRational fps = demuxer_get_framerate(demuxer);
    if (fps.num <= 0) fps = (AVRational){30, 1}; // fallback if r_frame_rate unavailable

    VideoEncoder* encoder = encoder_create("libx264", width, height, fps, 2000000, 1);
    CHECK(encoder != NULL, "encoder_create succeeds");
    if (!encoder) { decoder_free(decoder); demuxer_free(demuxer); return; }

    VideoMuxer* muxer = muxer_create(output_path, encoder, fps);
    CHECK(muxer != NULL, "muxer_create succeeds (header written)");
    if (!muxer) {
        encoder_free(encoder); decoder_free(decoder); demuxer_free(demuxer); return;
    }

    RawFrame* frame = frame_create();
    uint8_t* data; int size; int64_t pts;
    uint8_t* pkt_data; int pkt_size; int64_t pkt_pts; int64_t pkt_dts;
    int decoded = 0, encoded = 0, muxed = 0;
    int ret;

    while (demuxer_read_packet(demuxer, &data, &size, &pts) == 0) {
        if (decoder_send_packet(decoder, data, size, pts) < 0) break;
        while ((ret = decoder_receive_frame(decoder, frame)) == 0) {
            decoded++;
            if (encoder_send_frame(encoder, frame) == 0) {
                while (encoder_receive_packet(encoder, &pkt_data, &pkt_size, &pkt_pts, &pkt_dts) == 0) {
                    encoded++;
                    if (muxer_write_packet(muxer, pkt_data, pkt_size, pkt_pts, pkt_dts,
                            encoder->codec_ctx->time_base) == 0) {
                        muxed++;
                    }
                }
            }
        }
    }

    decoder_send_packet(decoder, NULL, 0, 0);
    while ((ret = decoder_receive_frame(decoder, frame)) == 0) {
        decoded++;
        if (encoder_send_frame(encoder, frame) == 0) {
            while (encoder_receive_packet(encoder, &pkt_data, &pkt_size, &pkt_pts, &pkt_dts) == 0) {
                encoded++;
                if (muxer_write_packet(muxer, pkt_data, pkt_size, pkt_pts, pkt_dts,
                        encoder->codec_ctx->time_base) == 0) {
                    muxed++;
                }
            }
        }
    }

    encoder_send_frame(encoder, NULL);
    while (encoder_receive_packet(encoder, &pkt_data, &pkt_size, &pkt_pts, &pkt_dts) == 0) {
        encoded++;
        if (muxer_write_packet(muxer, pkt_data, pkt_size, pkt_pts, pkt_dts,
                encoder->codec_ctx->time_base) == 0) {
            muxed++;
        }
    }

    printf("  decoded=%d encoded=%d muxed=%d\n", decoded, encoded, muxed);
    CHECK(decoded > 0, "at least one frame decoded");
    CHECK(encoded == muxed, "every encoded packet was successfully muxed");

    frame_free(frame);
    muxer_free(muxer);
    encoder_free(encoder);
    decoder_free(decoder);
    demuxer_free(demuxer);

    printf("  verify with: ffprobe -v error -select_streams v:0 -count_frames "
           "-show_entries stream=nb_read_frames -of csv=p=0 %s\n", output_path);
}

// ---------------------------------------------------------------------
int main(int argc, char** argv) {
    const char* path = (argc > 1) ? argv[1] : "../samples/input.mp4";
    int expected_frames = (argc > 2) ? atoi(argv[2]) : 0;
    const char* output_path = (argc > 3) ? argv[3] : "output.mp4";

    printf("Testing: %s\n", path);
    if (expected_frames > 0) {
        printf("Expected frame count (from ffprobe): %d\n", expected_frames);
    }

    test_open_and_metadata(path);
    test_full_decode(path, expected_frames);
    test_repeat_decode(path);
    test_invalid_inputs();
    test_frame_data_sanity(path);
    test_transcode_and_mux(path, output_path);

    printf("\n===================================\n");
    printf("TOTAL: %d passed, %d failed\n", tests_passed, tests_failed);
    printf("===================================\n");

    return tests_failed > 0 ? 1 : 0;
}
