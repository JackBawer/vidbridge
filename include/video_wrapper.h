#ifndef VIDEO_WRAPPER_H
#define VIDEO_WRAPPER_H

#ifdef __cplusplus
extern "C" {
#endif

#include <stdint.h>
#include <libavutil/rational.h>

// Opaque handle declarations
typedef struct VideoDemuxer VideoDemuxer;
typedef struct VideoDecoder VideoDecoder;
typedef struct VideoEncoder VideoEncoder;
typedef struct VideoMuxer VideoMuxer;
typedef struct RawFrame RawFrame;

// Frame API
RawFrame* frame_create(void);
void frame_free(RawFrame* frame);
uint8_t* frame_get_data(RawFrame* frame, int plane);
int frame_get_linesize(RawFrame* frame, int plane);

// Demuxer API
VideoDemuxer* demuxer_create(const char* url);
void demuxer_free(VideoDemuxer* demuxer);
int demuxer_read_packet(VideoDemuxer* demuxer, uint8_t** out_data, 
        int* out_size, int64_t* out_pts);
int demuxer_get_width(VideoDemuxer* demuxer);
int demuxer_get_height(VideoDemuxer* demuxer);
AVRational demuxer_get_framerate(VideoDemuxer* demuxer);
const char* demuxer_get_codec_name(VideoDemuxer* demuxer);

// Decoder API
VideoDecoder* decoder_create(const char* codec_name);
void decoder_free(VideoDecoder* decoder);
int decoder_initialize_from_demuxer(VideoDecoder* decoder,
        VideoDemuxer* demuxer);
int decoder_send_packet(VideoDecoder* decoder, const uint8_t* data, int size,
        int64_t pts);
int decoder_receive_frame(VideoDecoder* decoder, RawFrame* frame);

// Encoder API
VideoEncoder* encoder_create(const char* codec_name, int width, int height,
        AVRational fps, int bitrate, int needs_global_header);
void encoder_free(VideoEncoder* encoder);
int encoder_send_frame(VideoEncoder* encoder, RawFrame* frame);
int encoder_receive_packet(VideoEncoder* encoder, uint8_t** out_data,
        int* out_size, int64_t* out_pts, int64_t* out_dts);

// Muxer API
VideoMuxer* muxer_create(const char* output_path, VideoEncoder* encoder, 
        AVRational framerate);
void muxer_free(VideoMuxer* muxer);
int muxer_write_packet(VideoMuxer* muxer, uint8_t* data, int size, int64_t pts, 
        int64_t dts, AVRational encoder_time_base);

// Common
int vidbridge_averror_eagain(void);
int vidbridge_averror_eof(void);

AVRational encoder_get_time_base(VideoEncoder *enc);

#ifdef __cplusplus
}
#endif

#endif // !VIDEO_WRAPPER_H
