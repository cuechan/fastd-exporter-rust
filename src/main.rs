use clap;
use clap::Arg;
use fastd::FastdStatus;
#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};
use pretty_env_logger;
use prometheus::{Encoder, GaugeVec, Opts, TextEncoder};
use serde_json as json;
use std::boxed::Box;
use std::clone::Clone;
use std::env;
use std::io::Read;
use std::net::SocketAddr;
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::process::exit;
use std::str::FromStr;
use tiny_http::{self, Method, Response, Server};

mod fastd;

const DEFAULT_LISTEN_ADDR: &str = "0.0.0.0:9281";


fn main() {
	// env::set_var("FASTD", "debug");
	pretty_env_logger::init();

	let matches = clap::App::new(env!("CARGO_PKG_NAME"))
		.author(env!("CARGO_PKG_AUTHORS"))
		.about(env!("CARGO_PKG_DESCRIPTION"))
		.arg(
			Arg::with_name("socket")
				.takes_value(true)
				.conflicts_with("interface")
				.long("socket")
				.short("s")
				.help("Path to status socket")
				.group("status_socket")
				// following allows `--sock /tmp/vpn01 --sock /tmp/vpn02` but not --sock /tmp/vpn01,/tmp/vpn02
				.number_of_values(1)
				.multiple(true)
				.required(true)
				.validator(|s| match PathBuf::from(s).exists() {
					true => Ok(()),
					false => Err("socket does not exists".to_string()),
				}),
		)
		.arg(
			Arg::with_name("listen-address")
				.takes_value(true)
				.long("--web.listen-address")
				.help("Listen address for http server")
				.default_value(DEFAULT_LISTEN_ADDR)
				.validator(|v| SocketAddr::from_str(&v).map_err(|e| format!("{}", e)).map(|_| ())),
		)
		.get_matches();

	// if !matches.is_present("status_socket") {
	// 	eprintln!("interface or socket required. Try {} --help", env!("CARGO_PKG_NAME"));
	// 	exit(1);
	// }

	let socket_paths: Vec<PathBuf> = matches.values_of("socket").unwrap().map(|x| PathBuf::from(x)).collect();

	start_server(
		matches.value_of("listen-address").unwrap().parse().unwrap(),
		socket_paths,
	);
}

pub fn start_server(listen_addr: SocketAddr, fastd_sockets: Vec<PathBuf>) {
	let http_server = Server::http(listen_addr).unwrap();

	for request in http_server.incoming_requests() {
		let socket_paths = fastd_sockets.clone();

		if request.method() != &Method::Get || request.url() != "/metrics" {
			request
				.respond(Response::from_string(include_str!("index.html")))
				.unwrap();
			continue;
		}

		let mut instance_data = vec![];

		for ref socket in &socket_paths {
			instance_data.push(get_fastd_stats(socket));
		}

		let metrics = get_metrics(instance_data);
		request.respond(Response::from_data(metrics)).unwrap();
	}
}

pub fn get_metrics(fastd_statuses: Vec<FastdStatus>) -> Vec<u8> {
	let reg = prometheus::Registry::new();

	let peer_statistics_gauge: GaugeVec = GaugeVec::new(
		Opts::new("fastd_peer_traffic", "per peer statistics"),
		&["key", "name", "iface", "type", "kind"],
	).unwrap();
	reg.register(Box::new(peer_statistics_gauge.clone())).unwrap();

	let peer_uptime: GaugeVec = GaugeVec::new(
		Opts::new("fastd_peer_connection_uptime", "per peer statistics"),
		&["key", "name", "interface"],
	).unwrap();
	reg.register(Box::new(peer_uptime.clone())).unwrap();


	let fastd_statistics = GaugeVec::new(
		Opts::new("fastd_total_traffic", "total traffic"),
		&["iface", "type", "kind"],
	).unwrap();

	reg.register(Box::new(fastd_statistics.clone())).unwrap();

	let uptime: GaugeVec = GaugeVec::new(
		Opts::new("fastd_total_uptime", "fastd uptime"),
		&["iface"]
	).unwrap();

	for instance in fastd_statuses {
		uptime.with_label_values(&[&instance.interface]).set(instance.uptime);

		for (public_key, peer) in instance.peers.into_iter().filter(|(_, y)| y.connection.is_some()) {
			peer_uptime
				.with_label_values(&[&public_key, &peer.name, &instance.interface])
				.set(peer.connection.clone().unwrap().established);

			for (ref typ, ref stats) in peer.connection.unwrap().statistics {
				peer_statistics_gauge
					.with_label_values(&[&public_key, &peer.name, &instance.interface, &typ, "bytes"])
					.set(stats.bytes);

				peer_statistics_gauge
					.with_label_values(&[&public_key, &peer.name, &instance.interface, &typ, "packets"])
					.set(stats.packets);
			}
		}

		for (typ, traffic) in instance.statistics.into_iter() {
			fastd_statistics
				.with_label_values(&[&instance.interface, &typ, "bytes"])
				.set(traffic.bytes);

			fastd_statistics
				.with_label_values(&[&instance.interface, &typ, "packets"])
				.set(traffic.packets);
		}
	}

	let metrics = reg.gather();
	let mut buffer = Vec::new();
	TextEncoder::new().encode(&metrics, &mut buffer).unwrap();

	return buffer;
}

pub fn get_fastd_stats(path: &PathBuf) -> FastdStatus {
	let mut socket = UnixStream::connect(path.clone()).unwrap_or_else(|e| {
		error!("can't connect to {}: {}", path.to_string_lossy(), e);
		exit(1);
	});

	let mut status_json = String::new();

	socket.read_to_string(&mut status_json).unwrap_or_else(|e| {
		eprintln!("can't read from {}: {}", path.to_string_lossy(), e);
		exit(1);
	});

	let status: fastd::FastdStatus = json::from_str(&status_json).unwrap_or_else(|e| {
		eprintln!("can't parse: {}", e);
		exit(1);
	});

	return status;
}
