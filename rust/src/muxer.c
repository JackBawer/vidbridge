#include "internal.h"
#include "video_wrapper.h"
#include <libavcodec/avcodec.h>
#include <libavcodec/packet.h>
#include <libavformat/avformat.h>
#include <libavformat/avio.h>
#include <libavutil/avutil.h>
#include <stdlib.h>

VideoMuxer* muxer_create(const char* output_path, VideoEncoder* encoder, 
        AVRational framerate) {
    if (!output_path || !encoder || !encoder->codec_ctx) {
        return NULL;
    }

    VideoMuxer* muxer = (VideoMuxer*)malloc(sizeof(VideoMuxer));
    if (!muxer) {
        return NULL;
    }
    muxer->fmt_ctx = NULL;
    muxer->stream = NULL;
    muxer->is_open = 0;
    muxer->framerate = framerate;
    muxer->packet_count = 0;

    if (avformat_alloc_output_context2(&muxer->fmt_ctx, NULL, NULL, output_path) 
                || !muxer->fmt_ctx) {
        free(muxer);
        return NULL;
    }

    muxer->stream = avformat_new_stream(muxer->fmt_ctx, NULL);
    if (!muxer->stream) {
        avformat_free_context(muxer->fmt_ctx);
        free(muxer);
        return NULL;
    }

    // Copy codec parameters (SPS/PPS/extradata, etc.) from the already-open
    // encoder into this stream's codecpar, so the container header carries
    // the same info the decoder side needed - same principle as
    // avcodec_parameters_to_context, just the mirror direction.
    if (avcodec_parameters_from_context(muxer->stream->codecpar, 
                encoder->codec_ctx) < 0) {
        avformat_free_context(muxer->fmt_ctx);
        free(muxer);
        return NULL;
    }
    muxer->stream->time_base = encoder->codec_ctx->time_base;

    if (!(muxer->fmt_ctx->oformat->flags & AVFMT_NOFILE)) {
        if (avio_open(&muxer->fmt_ctx->pb, output_path, AVIO_FLAG_WRITE) < 0) {
            avformat_free_context(muxer->fmt_ctx);
            free(muxer);
            return NULL;
        }
    }

    if (avformat_write_header(muxer->fmt_ctx, NULL) < 0) {
        if (!(muxer->fmt_ctx->oformat->flags & AVFMT_NOFILE)) {
            avio_closep(&muxer->fmt_ctx->pb);
        }
        avformat_free_context(muxer->fmt_ctx);
        free(muxer);
        return NULL;
    }

    muxer->is_open = 1;
    return muxer;
}

void muxer_free(VideoMuxer* muxer) {
    if (!muxer) {
        return;
    }
    if (muxer->is_open) {
        av_write_trailer(muxer->fmt_ctx);
    }
    if (muxer->fmt_ctx) {
        if (!(muxer->fmt_ctx->oformat-> flags & AVFMT_NOFILE) 
                && muxer->fmt_ctx->pb) {
            avio_closep(&muxer->fmt_ctx->pb);
        }
        avformat_free_context(muxer->fmt_ctx);
    }
    free(muxer);
}

int muxer_write_packet(VideoMuxer* muxer, uint8_t* data, int size, int64_t pts,
        int64_t dts, AVRational encoder_time_base) {
    if (!muxer || !muxer->is_open || !data) {
        return -1;
    }

    AVPacket* pkt = av_packet_alloc();
    if (!pkt) {
        return -1;
    }

    // We copy here rather than borrow, because av_interleaved_write _frame
    // takes ownership semantics that don't match  the zero-copy contract
    // encoder_raceive_packet promises the caller (valid only until the next
    // receive_packet/free call) — safer to make this boundary an explicit copy
    // than stretch that lifetime further.
    if (av_new_packet(pkt, size) < 0) {
        av_packet_free(&pkt);
        return -1;
    }
    memcpy(pkt->data, data, size);
    pkt->pts = pts;
    pkt->dts = dts;
    pkt->duration = 1;
    pkt->stream_index = muxer->stream->index;

    av_packet_rescale_ts(pkt, encoder_time_base, muxer->stream->time_base);

    int ret = av_interleaved_write_frame(muxer->fmt_ctx, pkt);
    av_packet_free(&pkt);

    if (ret == 0) {
        muxer->packet_count++;
    }
    return ret;
}
