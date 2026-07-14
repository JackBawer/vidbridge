#include "internal.h"
#include "video_wrapper.h"
#include <libavformat/avformat.h>
#include <libavutil/avutil.h>
#include <stdlib.h>
#include <string.h>

VideoDemuxer* demuxer_create(const char* url) {
    VideoDemuxer* demuxer = (VideoDemuxer*)malloc(sizeof(VideoDemuxer));
    if (!demuxer) {
        return NULL;
    }

    demuxer->fmt_ctx = NULL;
    demuxer->pkt = av_packet_alloc();
    demuxer->stream_idx = -1;

    // Open the source (File or RTSP)
    if (avformat_open_input(&demuxer->fmt_ctx, url, NULL, NULL) < 0) {
        av_packet_free(&demuxer->pkt);
        free(demuxer);
        return NULL;
    }

    // Read header information to find streams
    if (avformat_find_stream_info(demuxer->fmt_ctx, NULL) < 0) {
        avformat_close_input(&demuxer->fmt_ctx);
        av_packet_free(&demuxer->pkt);
        free(demuxer);
        return NULL;
    }

    for (unsigned int i = 0; i < demuxer->fmt_ctx->nb_streams; ++i) {
        if (demuxer->fmt_ctx->streams[i]->codecpar->codec_type == 
                AVMEDIA_TYPE_VIDEO) {
            demuxer->stream_idx = (int)i;
            break;
        }
    }

    if (demuxer->stream_idx == -1) {
        avformat_close_input(&demuxer->fmt_ctx);
        av_packet_free(&demuxer->pkt);
        free(demuxer);
        return NULL;
    }

    return demuxer;
}

void demuxer_free(VideoDemuxer* demuxer) {
    if (!demuxer) {
        return;
    }
    if (demuxer->fmt_ctx) {
        avformat_close_input(&demuxer->fmt_ctx);
    }
    if (demuxer->pkt) {
        av_packet_free(&demuxer->pkt);
    }
    free(demuxer);
}

int demuxer_read_packet(VideoDemuxer* demuxer, uint8_t** out_data, 
        int* out_size, int64_t* out_pts) {
    av_packet_unref(demuxer->pkt);

    // Loop until we find a VIDEO packet (ignore audio/subtitles) 
    while (av_read_frame(demuxer->fmt_ctx, demuxer->pkt) >= 0) {
        if (demuxer->pkt->stream_index == demuxer->stream_idx) {
            // Handoff: point the caller's pointer to our internal packet 
            // buffer
            *out_data = demuxer->pkt->data;
            *out_size = demuxer->pkt->size;
            *out_pts = demuxer->pkt->pts;
            return 0;
        }
        av_packet_unref(demuxer->pkt);
    }
    return -1;
}

int demuxer_get_width(VideoDemuxer* demuxer) {
    if (!demuxer || demuxer->stream_idx < 0 || demuxer->stream_idx >= demuxer->fmt_ctx->nb_streams) {
        return 0;
    }
    return demuxer->fmt_ctx->streams[demuxer->stream_idx]->codecpar->width;
}

int demuxer_get_height(VideoDemuxer* demuxer) {
    if (!demuxer || demuxer->stream_idx < 0 || demuxer->stream_idx >= demuxer->fmt_ctx->nb_streams) {
        return 0;
    }
    return demuxer->fmt_ctx->streams[demuxer->stream_idx]->codecpar->height;
}

AVRational demuxer_get_framerate(VideoDemuxer* demuxer) {
    if (!demuxer || demuxer->stream_idx < 0 || 
            (unsigned)demuxer->stream_idx >= demuxer->fmt_ctx->nb_streams) {
        return (AVRational){0, 1};
    }
    AVStream* stream = demuxer->fmt_ctx->streams[demuxer->stream_idx];
    if (stream->avg_frame_rate.num > 0 && stream->avg_frame_rate.den > 0) {
        return stream->avg_frame_rate;
    } 
    return stream->r_frame_rate;
}

const char* demuxer_get_codec_name(VideoDemuxer* demuxer) {
    if (!demuxer || demuxer->stream_idx < 0 || demuxer->stream_idx >= demuxer->fmt_ctx->nb_streams) {
        return "invalid_stream";
    }
    enum AVCodecID id = demuxer->fmt_ctx->streams[demuxer->stream_idx]->codecpar->codec_id;
    const char* name = avcodec_get_name(id);
    return name ? name : "unknown_codec";
}
