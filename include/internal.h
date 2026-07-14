#ifndef INTERNAL_H
#define INTERNAL_H

#include "video_wrapper.h"
#include <libavformat/avformat.h>
#include <libavcodec/avcodec.h>

// Definition of the opaque handles
struct RawFrame {
    AVFrame* av_frame;
};

struct VideoDemuxer {
    AVFormatContext* fmt_ctx;
    int stream_idx;
    AVPacket* pkt;
};

struct VideoDecoder {
    AVCodecContext* codec_ctx;
    const AVCodec* codec;
    int is_open;
    AVPacket* pkt;
};

struct VideoEncoder {
    AVCodecContext* codec_ctx;
    AVPacket* pkt;
    int64_t frame_count;
};

struct VideoMuxer {
    AVFormatContext* fmt_ctx;
    AVStream* stream;
    int is_open;
    AVRational framerate;
    int64_t packet_count;
};

#endif // !INTERNAL_H
