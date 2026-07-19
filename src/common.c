// src/common.c

#include "internal.h"

#include <errno.h>
#include <libavutil/error.h>

int vidbridge_averror_eagain(void)
{
    return AVERROR(EAGAIN);
}

int vidbridge_averror_eof(void)
{
    return AVERROR_EOF;
}

AVRational encoder_get_time_base(VideoEncoder *enc)
{
    if (!enc || !enc->codec_ctx) {
        AVRational tb = {0, 1};
        return tb;
    }

    return enc->codec_ctx->time_base;
}
