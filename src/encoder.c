#include "internal.h"
#include <libavcodec/avcodec.h>
#include <libavcodec/codec.h>
#include <libavutil/opt.h>
#include <stdlib.h>

VideoEncoder* encoder_create(const char* codec_name, int width, int height,
        AVRational fps, int bitrate, int needs_global_header) {

    const AVCodec* codec = avcodec_find_encoder_by_name(codec_name);
    if (!codec) {
        return NULL;
    }

    VideoEncoder* encoder = (VideoEncoder*)malloc(sizeof(VideoEncoder));
    if (!encoder) {
        return NULL;
    }

    encoder->codec_ctx = avcodec_alloc_context3(codec);
    if (!encoder->codec_ctx) {
        free(encoder);
        return NULL;
    }

    encoder->pkt = av_packet_alloc();
    if (!encoder->pkt) {
        avcodec_free_context(&encoder->codec_ctx);
        free(encoder);
        return NULL;
    }

    encoder->frame_count = 0;

    // Basic settings
    encoder->codec_ctx->width = width;
    encoder->codec_ctx->height = height;
    encoder->codec_ctx->bit_rate = bitrate;

    // Timebase is the "Clock" of the video
    // For 30fps, time_base is 1/30
    encoder->codec_ctx->time_base = av_inv_q(fps);
    encoder->codec_ctx->framerate = fps;
    encoder->codec_ctx->gop_size = 10; // Keyframe every 10 frames
    encoder->codec_ctx->pix_fmt = AV_PIX_FMT_YUV420P;

    // CRITICAL: required by many containers (MP4/MKV) so the codec's
    // header info (e.g. SPS/PPS) is stored once in the container's
    // global header instead of repeated in every packet. This must be
    // decided by the caller based on the target output format, since
    // codec_ctx->flags has nothing meaningful in it yet at this point.
    if (needs_global_header) {
        encoder->codec_ctx->flags |= AV_CODEC_FLAG_GLOBAL_HEADER;
    }

    // Set H.265 specific options (like preset speed)
    if (codec->id == AV_CODEC_ID_HEVC) {
        if (av_opt_set(encoder->codec_ctx->priv_data, "preset", "veryfast", 0) < 0) {
            // Non-fatal: encoder will just fall back to its default preset.
            fprintf(stderr, "warning: failed to set HEVC preset\n");
        }
    }

    if (avcodec_open2(encoder->codec_ctx, codec, NULL) < 0) {
        av_packet_free(&encoder->pkt);
        avcodec_free_context(&encoder->codec_ctx);
        free(encoder);
        return NULL;
    }

    return encoder;
}

void encoder_free(VideoEncoder* encoder) {
    if (!encoder) {
        return;
    }
    av_packet_free(&encoder->pkt);
    avcodec_free_context(&encoder->codec_ctx);
    free(encoder);
}

int encoder_send_frame(VideoEncoder* encoder, RawFrame* frame) {
    if (!encoder || !encoder->codec_ctx) {
        return -1;
    }
    if (frame && !frame->av_frame) {
        return -1;
    }

    if (frame) {
        // Sequential PTS assumes constant frame rate matching
        // codec_ctx->time_base (1/fps) set in encoder_create. If frames
        // can be dropped/duplicated upstream, this will drift silently.
        frame->av_frame->pts = encoder->frame_count++;
    }

    // frame == NULL is the documented flush signal: tells the encoder
    // no more input is coming, so it can start draining any frames it
    // buffered internally (e.g. for B-frame lookahead).
    return avcodec_send_frame(encoder->codec_ctx, frame ? frame->av_frame : NULL);
}

int encoder_receive_packet(VideoEncoder* encoder, uint8_t** out_data,
        int* out_size, int64_t* out_pts, int64_t* out_dts) {
    if (!encoder || !encoder->codec_ctx || !encoder->pkt ||
            !out_data || !out_size || !out_pts || !out_dts) {
        return AVERROR(EINVAL);
    }

    av_packet_unref(encoder->pkt);

    int ret = avcodec_receive_packet(encoder->codec_ctx, encoder->pkt);
    if (ret == 0) {
        // ZERO-COPY: valid until the next call to encoder_receive_packet
        // or encoder_free. Caller must copy out before either happens.
        *out_data = encoder->pkt->data;
        *out_size = encoder->pkt->size;
        *out_pts = encoder->pkt->pts;
        *out_dts = encoder->pkt->dts;
    }
    return ret;
}
