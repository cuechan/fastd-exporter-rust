use clap;
use clap::Arg;
use fastd::FastdStatus;
use hyper;
use hyper::rt::Future;
use hyper::service::service_fn;
use hyper::service::service_fn_ok;
use hyper::{header::CONTENT_TYPE, Body, Method, Request, Response, Server};
use log::{debug, error, info, trace, warn};
use pretty_env_logger;
use prometheus::core::{GenericGauge, MetricVec};
use prometheus::register;
use prometheus::{Counter, CounterVec, Encoder, Gauge, GaugeVec, HistogramVec, Opts, TextEncoder};
use serde_json as json;
use std::boxed::Box;
use std::clone::Clone;
use std::collections::HashMap;
use std::env;
use std::io::Read;
use std::net::SocketAddr;
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::process::exit;
use std::str::FromStr;
use std::time;
use zmq;

mod fastd;
mod zmq_service;

const DEFAULT_LISTEN_ADDR: &str = "0.0.0.0:9101";
const ZMQ_BIND_ADDRESS: &str = "tcp://0.0.0.0:25259";

macro_rules! map(
	{ $($key:expr => $value:expr),+ } => {
		{
			let mut map = ::std::collections::HashMap::new();
			$(
				map.insert($key, $value);
			)+
			map
		}
	 };
);

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

	if !matches.is_present("status_socket") {
		eprintln!("interface or socket required. Try {} --help", env!("CARGO_PKG_NAME"));
		exit(1);
	}

	let socket_paths: Vec<PathBuf> = matches.values_of("socket").unwrap().map(|x| PathBuf::from(x)).collect();

	// zmq_service::start_zmq_service(socket_paths);

	start_server(matches.value_of("listen-address").unwrap(), socket_paths);
}

pub fn start_server(listen_addr: &str, fastd_sockets: Vec<PathBuf>) {
	let new_service = move || {
		let socket_paths = fastd_sockets.clone();

		service_fn(move |r| {
			debug!(
				"[{}] {} {}",
				r.headers().get("User-Agent").unwrap().to_str().unwrap(),
				r.method(),
				r.uri()
			);

			if r.method() != Method::GET || r.uri().path() != "/metrics" {
				return Err("error");
			}

			let mut instance_data = vec![];

			for ref socket in &socket_paths {
				instance_data.push(get_fastd_stats(socket));
			}

			let metrics = get_metrics(instance_data);

			Ok(Response::builder()
				.status(200)
				.header(CONTENT_TYPE, "text/plain")
				.body(Body::from(metrics))
				.unwrap())
		})
	};

	let server = Server::bind(&SocketAddr::from_str(listen_addr).unwrap())
		.serve(new_service)
		.map_err(|e| panic!("error occured: {}", e));

	hyper::rt::run(server);
}

pub fn get_metrics(fastd_statuses: Vec<FastdStatus>) -> Vec<u8> {
	let reg = prometheus::Registry::new();

	let peer_statistics_gauge: GaugeVec = GaugeVec::new(
		Opts {
			namespace: "fastd".into(),
			subsystem: "peer".into(),
			name: "traffic".into(),
			help: "per peer statistics".into(),
			const_labels: HashMap::new(),
			variable_labels: vec![],
		},
		&["key", "name", "iface", "type", "kind"],
	)
	.unwrap();

	let peer_uptime: GaugeVec = GaugeVec::new(
		Opts {
			namespace: "fastd".into(),
			subsystem: "peer".into(),
			name: "uptime".into(),
			help: "per peer statistics".into(),
			const_labels: HashMap::new(),
			variable_labels: vec![],
		},
		&["key", "name", "interface"],
	)
	.unwrap();

	reg.register(Box::new(peer_statistics_gauge.clone())).unwrap();

	let fastd_statistics = GaugeVec::new(
		Opts {
			namespace: "fastd".to_owned(),
			subsystem: "total".to_owned(),
			name: "statistics".into(),
			help: "total traffic {}".into(),
			const_labels: HashMap::new(),
			variable_labels: vec!["iface".into()],
		},
		&["iface", "type", "kind"],
	)
	.unwrap();

	reg.register(Box::new(fastd_statistics.clone())).unwrap();

	let uptime: GaugeVec = GaugeVec::new(
		Opts {
			namespace: "fastd".to_owned(),
			subsystem: "total".to_owned(),
			name: "uptime".to_owned(),
			help: "fastd uptime".to_owned(),
			const_labels: HashMap::new(),
			variable_labels: vec!["iface".into()],
		},
		&["iface"],
	)
	.unwrap();

	//	eprintln!("{}", json::to_string_pretty(&fastd_status).unwrap());

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
