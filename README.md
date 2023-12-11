# crunch64

## C bindings

This library provides bindings to call this library from C code. They are available on the [releases](https://github.com/decompals/crunch64/releases) tab.

To build said bindings from source, enable the `c_bindings` Rust feature:

```bash
cargo build --lib --features c_bindings
```

Headers are located at [c_bindings/include](c_bindings/include).

### Windows executables

Due to Rust requirements, linking the C bindings of this library when building a C program adds extra library dependencies. Those libraries are the following:

```plain_text
-lws2_32 -lntdll -lbcrypt -ladvapi32 -luserenv
```
