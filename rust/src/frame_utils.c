#include "internal.h"
#include "video_wrapper.h"
#include <stdlib.h>

RawFrame* frame_create(void) {
    RawFrame* frame = (RawFrame*)malloc(sizeof(RawFrame));
    if (!frame) {
        return NULL;
    }
    frame->av_frame = av_frame_alloc();
    if (!frame->av_frame) {
        free(frame);
        return NULL;
    }

    return frame;
}

void frame_free(RawFrame* frame) {
    if (!frame) {
        return;
    }
    if (frame->av_frame) {
        av_frame_free(&frame->av_frame);
    }
    free(frame);
}

uint8_t* frame_get_data(RawFrame* frame, int plane) {
    if (!frame || !frame->av_frame || plane < 0 || plane >= 8) {
        return NULL;
    }
    return frame->av_frame->data[plane];
}

int frame_get_linesize(RawFrame *frame, int plane) {
    if (!frame || !frame->av_frame || plane < 0 || plane >= 8) {
        return -1;
    }
    return frame->av_frame->linesize[plane];
}
