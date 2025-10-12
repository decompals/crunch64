#ifndef CRUNCH64_ERROR_H
#define CRUNCH64_ERROR_H
#pragma once

#ifdef __cplusplus
extern "C"
{
#endif

/* This needs to be synced with the Rust equivalent in `src/lib.rs` */
typedef enum Crunch64Error {
    Crunch64Error_Okay,
    Crunch64Error_InvalidYay0Header,
    Crunch64Error_InvalidYaz0Header,
    Crunch64Error_InvalidMio0Header,
    Crunch64Error_UnsupportedCompressionType,
    Crunch64Error_UnalignedRead,
    Crunch64Error_ByteConversion,
    Crunch64Error_OutOfBounds,
    Crunch64Error_NullPointer,
    Crunch64Error_InvalidCompressionLevel,
    Crunch64Error_Vpk0,
} Crunch64Error;

#ifdef __cplusplus
}
#endif

#endif
