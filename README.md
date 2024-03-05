

# modbus-mapping

An API for Modbus Register Mapppings based on [tokio-modbus](https://github.com/slowtec/tokio-modbus)

[![Docs.rs](https://docs.rs/modbus-mapping/badge.svg)](https://docs.rs/modbus-mapping/) [![crates.io](https://img.shields.io/crates/v/modbus-mapping)](https://crates.io/crates/modbus-mapping)

[Repo URL](https://github.com/vladimirvrabely/modbus-mapping)


## Usage

Check [examples](modbus-mapping/examples/) folder for usage. 

The device and client examples pair is to be run at the same time in two different terminals. The RTU examples assume that virtual serial port has been created beforehand
```bash
socat -d -d pty,raw,echo=0,link=/tmp/ttys001 pty,raw,echo=0,link=/tmp/ttys002
```


## Development

Check [`justfile`](justfile).
