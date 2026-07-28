#include "internal.h"
#include "video_wrapper.h"
#include <libavcodec/avcodec.h>
#include <libavcodec/codec.h>
#include <libavutil/mem.h>
#include <stdlib.h>
#include <string.h>

VideoDecoder* decoder_create(const char* codec_name) {
    const AVCodec* codec = avcodec_find_decoder_by_name(codec_name);
    if (!codec) {
        return NULL;
    }

    VideoDecoder* decoder = (VideoDecoder*)malloc(sizeof(VideoDecoder));
    if (!decoder) {
        return NULL;
    }

    decoder->codec_ctx = avcodec_alloc_context3(codec);
    if (!decoder->codec_ctx) {
        free(decoder);
        return NULL;
    }

    decoder->pkt = av_packet_alloc();
    if (!decoder->pkt) {
        avcodec_free_context(&decoder->codec_ctx);
        free(decoder);
        return NULL;
    }

    decoder->codec = codec;
    decoder->is_open = 0;

    return decoder;
}

void decoder_free(VideoDecoder* decoder) {
    if (!decoder) {
        return;
    }

    if (decoder->codec_ctx) {
        avcodec_free_context(&decoder->codec_ctx);
    }

    if (decoder->pkt) {
        av_packet_free(&decoder->pkt);
    }
    free(decoder);
}

int decoder_initialize_from_demuxer(VideoDecoder* decoder, 
        VideoDemuxer* demuxer) {
    if (!decoder || !decoder->codec_ctx || !demuxer || !demuxer->fmt_ctx) {
        return AVERROR(EINVAL);
    }
    if (demuxer->stream_idx < 0 || 
            (unsigned)demuxer->stream_idx >= demuxer->fmt_ctx->nb_streams) {
        return AVERROR(EINVAL);
    }
    int ret = avcodec_parameters_to_context(decoder->codec_ctx, 
            demuxer->fmt_ctx->streams[demuxer->stream_idx]->codecpar);

    if (ret < 0) {
        return ret;
    }

    ret = avcodec_open2(decoder->codec_ctx, decoder->codec, NULL);
    if (ret < 0) {
        return ret;
    }
    decoder->is_open = 1;
    return 0;
}

int decoder_initialize_with_extradata(VideoDecoder* decoder,
        const uint8_t* extradata, int extradata_size) {
    if (!decoder || !decoder->codec_ctx || !extradata || extradata_size <= 0) {
        return AVERROR(EINVAL);
    }

    // FFmpeg requires AV_INPUT_BUFFER_PADDING_SIZE extra zeroed bytes past
    // the end of extradata, and takes ownership of the buffer once set.
    uint8_t* buf = (uint8_t*)av_mallocz(extradata_size + AV_INPUT_BUFFER_PADDING_SIZE);
    if (!buf) {
        return AVERROR(ENOMEM);
    }
    memcpy(buf, extradata, extradata_size);

    decoder->codec_ctx->extradata = buf;
    decoder->codec_ctx->extradata_size = extradata_size;

    int ret = avcodec_open2(decoder->codec_ctx, decoder->codec, NULL);
    if (ret < 0) {
        return ret;
    }
    decoder->is_open = 1;
    return 0;
}

int decoder_send_packet(VideoDecoder* decoder, const uint8_t* data, int size,
        int64_t pts) {
    if (!decoder || !decoder->is_open || !decoder->pkt) {
        return -1;
    }

    av_packet_unref(decoder->pkt);

    if (data) {
        decoder->pkt->data = (uint8_t*)data;
        decoder->pkt->size = size;
        decoder->pkt->pts = pts;
        return avcodec_send_packet(decoder->codec_ctx, decoder->pkt);
    }
    return avcodec_send_packet(decoder->codec_ctx, NULL);
}

int decoder_receive_frame(VideoDecoder* decoder, RawFrame* frame) {
    if (!decoder || !decoder->codec_ctx || !frame || !frame->av_frame) {
        return AVERROR(EINVAL);
    }
    return avcodec_receive_frame(decoder->codec_ctx, frame->av_frame);
}
