[prometheus-fastd-exporter](https://freifunk-luebeck.pages.chaotikum.org/prometheus-fastd-exporter/)
=============================
[![pipeline status](https://git.chaotikum.org/paul/prometheus-fastd-exporter/badges/master/pipeline.svg)](https://git.chaotikum.org/paul/prometheus-fastd-exporter/-/commits/master)
[![latest .deb](https://img.shields.io/badge/Debian-requestd.deb-%23CE0056)](https://freifunk-luebeck.pages.chaotikum.org/prometheus-fastd-exporter/prometheus-fastd-exporter.deb)

A prometheus exporter for fastd.


Debian package
--------------
[![latest .deb](https://img.shields.io/badge/Debian-requestd.deb-%23CE0056)](https://freifunk-luebeck.pages.chaotikum.org/prometheus-fastd-exporter/prometheus-fastd-exporter.deb)


After installing it with `dpgk -i prometheus-fastd-exporter.deb` you need to reload systemd: `sudo systemctl daemon-relaod`.
The exporter is not enabled by default. Enable it with `sudo systemctl enable prometheus-fastd-exporter`.


Building
--------

`cargo build --release` to build the project.
The binary is located in `target/release/prometheus-fastd-exporter`.

To build a debian package install `cargo-deb` with `cargo install cargo-deb` and run `cargo deb`.
The package is then, depending on the version, located in `target/debian/fastd-exporter-rust_<version>_amd64.deb`


Running
-------
Simply run `./fastd-exporter-rust -s <socket>`. You can export data from multiple fastd instances by repeating the `-s <socket>` option.
