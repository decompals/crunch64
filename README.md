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

## Python bindings

This library provides bindings to call this library from Python code. The recommended way to install this library is via PyPI (Check out this project at PyPI <https://pypi.org/project/crunch64/>):

```bash
python3 -m pip install -U crunch64
```

### Development version

The development version is located in the Github repository. In case the user wants to get the latest and unreleased features then they can install the repo directly.

Please note building the Python version from source requires the Rust toolchain and the `maturin` Python package.

The recommended way to install a locally cloned repo is by using `pip`.

```bash
python3 -m pip install ./lib
```

In case you want to mess with the latest development version without wanting to clone the repository, then you could use the following commands:

```bash
python3 -m pip uninstall crunch64
python3 -m pip install "git+https://github.com/decompals/crunch64.git#egg=crunch64&subdirectory=lib"
```

NOTE: Installing the development version is not recommended unless you know what you are doing. Proceed at your own risk.

## References

- Yaz0
  - Reference implementation by Mr-Wiseguy: <https://gist.github.com/Mr-Wiseguy/6cca110d74b32b5bb19b76cfa2d7ab4f>
- MIO0
  - Hack64.net docs: <https://hack64.net/wiki/doku.php?id=super_mario_64:mio0>
