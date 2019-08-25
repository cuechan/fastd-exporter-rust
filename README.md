Fastd exporter for prometheus
=============================

Building
--------

`cargo build --release` to build the project.
The binary is then located in `target/release/fastd-exporter-rust`.

To build a `.deb`ian package install `cargo-deb` with `cargo install cargo-deb` and run `cargo deb` to build the package.
The package is then, depending on the version, located in `target/debian/fastd-exporter-rust_<version>_amd64.deb`


Running
-------
Just run `./fastd-exporter-rust -i <interface>` or `./fastd-exporter-rust -s <socket>`.
When using `-i <interafce>` the socket is expected to be at `/var/run/fastd.<interface>.sock`.

The http listen address is currently hardcoded to `0.0.0.0:9101`.

Todo
----

* [ ] set http listening address with a `-l` or `--listen` argument.
* [ ] change binary name to `prometheus-fastd-exported` since this is the naming convention
* [ ] add more metrics?
