use clap;
use clap::Arg;
use fastd::FastdStatus;
use hyper;
use hyper::{header::CONTENT_TYPE, Body, Response, Server, Request, Method};
use hyper::rt::Future;
use hyper::service::service_fn;
use hyper::service::service_fn_ok;
use prometheus::{Counter, CounterVec, Gauge, GaugeVec, Encoder, HistogramVec, TextEncoder, Opts};
use prometheus::core::{GenericGauge, MetricVec};
use prometheus::{register};
use serde_json as json;
use std::clone::Clone;
use std::io::Read;
use std::net::SocketAddr;
use std::os::unix::net::UnixStream;
use std::path::Path;
use std::process::exit;
use std::str::FromStr;
use std::time;
use std::boxed::Box;
use std::collections::HashMap;

mod fastd;

const LISTEN_ADDR: &str = "0.0.0.0:9101";


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
	let matches = clap::App::new(env!("CARGO_PKG_NAME"))
		.author(env!("CARGO_PKG_AUTHORS"))
		.about(env!("CARGO_PKG_DESCRIPTION"))
		.arg(Arg::with_name("iface")
			.takes_value(true)
			.long("iface")
			.short("i")
			.conflicts_with("socket-address")
			.help("Name of the fastd interface. We expect the \nstatus socket at /var/run/fastd.<interface>.sock")
			.group("status_socket")
		)
		.arg(Arg::with_name("socket")
			.takes_value(true)
			.conflicts_with("interface")
			.long("socket")
			.short("s")
			.help("Path to status socket")
			.group("status_socket")
		)
		.get_matches();

	if !matches.is_present("status_socket") {
		eprintln!("interface or socket required. Try {} --help", env!("CARGO_PKG_NAME"));
		exit(1);
	}


	let socket_path = match (matches.value_of("iface"), matches.value_of("socket")) {
		(Some(iface), None) => format!("/var/run/fastd.{}.sock", iface.clone()),
		(None, Some(path)) => path.to_owned(),
		_ => panic!("either iface nor socket passed")
	};


	let new_service = move || {
		let socket_path = socket_path.clone();

		service_fn(move |r| {
			println!("[{}] {} {}", r.headers().get("User-Agent").unwrap().to_str().unwrap(), r.method(), r.uri());

			if r.method() != Method::GET || r.uri().path() != "/metrics" {
				return Err("error");
			}


			let metrics = get_metrics(&socket_path);

			Ok(Response::builder()
				.status(200)
				.header(CONTENT_TYPE, "text/plain")
				.body(Body::from(metrics))
				.unwrap()
			)
		})
    };

    let server = Server::bind(&SocketAddr::from_str(LISTEN_ADDR).unwrap())
        .serve(new_service)
		.map_err(|e| {
			panic!("error occured: {}", e)
		});

    hyper::rt::run(server);
}



pub fn get_metrics<T: AsRef<Path>>(path: T) -> Vec<u8> {
	let reg = prometheus::Registry::new();
	let fastd_status = get_fastd_stats(&path.as_ref().to_str().unwrap());

	eprintln!("{}", json::to_string_pretty(&fastd_status).unwrap());



	for (public_key, peer) in fastd_status.peers
		.into_iter()
		.filter(|(_,y)| y.connection.is_some())
	{
		// `IntoIter` trait is implemented for `Connection`

		for (metric, traffic) in peer.connection.unwrap().statistics.into_iter() {
			let mut traffic_gauge = Gauge::with_opts(Opts {
				namespace: "fastd".to_owned(),
				subsystem: "peer".to_owned(),
				name: metric,
				help: "does things".to_owned(),
				const_labels: map!{
					"key".to_owned()       => public_key.clone(),
					"name".to_owned()      => peer.name.clone(),
					"interface".to_owned() => fastd_status.interface.clone()
				},
				variable_labels: vec![],
			}).unwrap();

			reg.register(Box::new(traffic_gauge.clone())).unwrap();

			traffic_gauge.set(traffic.bytes)
		}

	}





	let metrics = reg.gather();
	let mut buffer = Vec::new();
	TextEncoder::new().encode(&metrics, &mut buffer).unwrap();

	return buffer;
	"Hello, prometheus!".bytes().collect()
}




pub struct PeerMetric {
	peer_key: String,
	name: String,
}







pub fn get_fastd_stats(path: &str) -> FastdStatus {
	let mut socket = UnixStream::connect(path.clone()).unwrap_or_else(|e| {
		eprintln!("can't connect to {}: {}", path, e);
		exit(1);
	});

	let mut status_json = String::new();

	socket.read_to_string(&mut status_json).unwrap_or_else(|e| {
		eprintln!("can't read from {}: {}", path, e);
		exit(1);
	});


	let status: fastd::FastdStatus = json::from_str(&status_json).unwrap_or_else(|e| {
		eprintln!("can't parse: {}", e);
		exit(1);
	});

	return status
}
