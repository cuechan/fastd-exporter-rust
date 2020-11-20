use zmq;
use zmq::Message;
use zmq::Socket;
use zmq::SocketType;

const ZMQ_BIND_ADDRESS: &str = "tcp://localhost:25259";

pub fn main() {
	let ctx = zmq::Context::new();
	let sock: Socket = ctx.socket(SocketType::REQ).unwrap();

	sock.connect(crate::ZMQ_BIND_ADDRESS).unwrap();

	loop {
		sock.send(Message::from("get something ..."), 0).unwrap();
		let req = sock.recv_string(0).unwrap().unwrap();
		println!("{:#?}", req);
	}
}
