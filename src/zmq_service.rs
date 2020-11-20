use std::path::PathBuf;
use zmq;
use zmq::Message;
use zmq::Socket;
use zmq::SocketType;

const ZMQ_BIND_ADDRESS: &str = "tcp://0.0.0.0:25259";

// pub fn start_zmq_service(fastd_sockets: Vec<PathBuf>) {
// 	let ctx = zmq::Context::new();

// 	let sock: Socket = ctx.socket(SocketType::REP).unwrap();

// 	sock.bind(crate::ZMQ_BIND_ADDRESS).unwrap();

// 	loop {
// 		let req = sock.recv_string(0).unwrap().unwrap();
// 		println!("{:#?}", req);

// 		crate::get_fastd_stats(PathBuf::from("/tmp/ffhl.sock"));
// 		sock.send_msg(Message::from("foobar"), 0).unwrap();
// 	}
// }
