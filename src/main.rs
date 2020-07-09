mod ext;
mod protocol;

use ext::*;
use protocol::{
	handshake::Handshake,
	login::{Disconnect, LoginStart, LoginSuccess, SetCompression},
	status::{Ping, Pong, StatusRequest, StatusResponse},
	Packet, PacketData, State,
};
use quick_error::quick_error;
use tokio::io;
use tokio::net::{TcpListener, TcpStream};

const THRESHOLD: i32 = 256;

struct LoggedInInfo {
	username: String,
	uuid: String,
}
quick_error! {
	#[derive(Debug)]
	pub enum SocketLoginError {
		Io(err: io::Error) {
			from()
			display("io error: {}", err)
			cause(err)
		}
		IncorrectStateIdCombo(state: State, id: i32) {}
	}
}
/// Проводит авторизацию юзера/выходит при ошибке/запросе статуса
async fn handle_socket_login(
	mut stream: TcpStream,
) -> Result<(TcpStream, LoggedInInfo), SocketLoginError> {
	let mut state = State::Handshaking;
	loop {
		let mut initial_buffer = Vec::new();
		let data = stream.read_packet(None, &mut initial_buffer).await?;
		match (state, data.id()) {
			(State::Handshaking, Handshake::ID) => {
				let packet = data.cast::<Handshake>()?;
				println!("Handshake: {:?}", packet);
				state = packet.next_state;
			}
			(State::Status, StatusResponse::ID) => {
				let req = data.cast::<StatusRequest>()?;
				println!("Request: {:?}", req);
				stream
					.write_packet(
						None,
						&StatusResponse {
							response: r#"
							{
								"version": {
									"name": "Cristalix",
									"protocol": 340
								},
								"players": {
									"max": 100,
									"online": 20,
									"sample": [
										{
											"name": "Привет мир",
											"id": "d6a33537-0444-45be-b12b-af138b1ab81f"
										}
									]
								},
								"description": {
									"text": "Hello world"
								},
								"favicon": "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAEgAAABICAMAAABiM0N1AAAAwFBMVEVHcEzdLkR3slWqjdh3slV3slV3slWXpm6qjdh3slV3slV/pHLULUP/zE3bSUnSJz3gNkx3slWhCyfdL0XdLkSqjdiZcdB3slVckTuqjdjcLUN3slWgByKqjdiSZsz/zE3/zE3eMEahBB6qjdigBB6lIUWgBB6gBB7qWW7qWW7dLkR3slXnUWbjRFn/zE3qWW7fN02qjdigBB6jI0iobKvMJTynCSOlQnOmVY2pgMeSZsxckTvsYmvUKD76rVO7GTF6PeTeAAAAKnRSTlMA6Jo7fr/YEL/vWSAQvyA9gTD6n19oUK+/h8tAW9PfUN+xeqmfx9qP58959arBAAAEJUlEQVR4Xr2Y2ZqiSBhEU2RRsN2Rdqsudapnms299up5/7cacjUNSJqriZu6O98fhz8TKfK/pr0+rudBI0BgdZOka3nVoN2RZt6AM+omLN1RJahzbEpyExG3cqbZkUe1+/GY548/yhw/UbGrm89Yu1BycpYyKaKtfJ/2c8sUrx2G7VAHPXLQI8FQgiUGGyFmflRpC1AuQjAUsBR/feCsb5wO+QNoJEFOWZI2z9ojf6jmyUoRr6hHDdThguplu0KyXYCcexBl7AjE+Pg92/YMoA4r1ZlTz81jaaD+ajvoERKqao1RQnbEOdsTzc8V2cHDhy7f8vwbngb9qU2fTzzj7Vx/+piCU5DK0hVowShiqtl8dztq3vePj+/e/Ub1EaQWcnA9aXnqk7nqVnAK0j0IOYFcTLKJFeXlTUliLj5YgtpqSwqi5vpxLByNL3BCBMgsW65RlxIncfwvB73CCcFq9WvUa8XxFwcpzNvrihCQbYwrjtowjsVIZ8p4GZ/P5wK6Mm0QxpMPrRhIjsSHeRmznn/XbRC6HhEypRxhSbi+8KJ/cQZukMn1gnEO6TPtJh7+SZAQVHfSBjFLuj8wSxfW7gSk2mriJbLhA+33e74C59fL5XwCklm2uvr73NC+CC0HAePEpMily0iT7WmyU0VW+LLyKkAWXUZWjOdaQ2qHu86x6toL3MQNyFCYphGaMOMB8cLdEULI7OH9/WEmca1bMaOmJ0ZB0OydRZCm0rSKJMG9glmTBw564KCFLKaS3ZPOGqYzD9s8YeiRdxHGGQjTer500kVNgC+oe9BGmVY5xJQE47w+EbxU9Gp8GTPgxDcS57zRy2BF8JrTZU9KxTIm/ypIfJox24EegRtce/y9FhZLKYbNJA29yWO31UAEMhSmkaNI4xd+xfGRzDd4q+CkIEiRSju+Nd7gU2YaOSpI+klMWUCxTKNI5XoGBs6g4PzO89+fyNFEYTeMXMaCU5BQtF7v+Q4U2JHj2AFRkcuYs3wCx2Rq64kPD19BxDJmHASijf1WS0pBUq9VmBbVUBCieMEnEkhQdL+M6f5TyEZOmXW9TnqE+E5ku5R0t4wZbHRdWsMe4VlSkKsv46E5pzUVFI/Pk1j6Mqaw0cYsBqXPxUBbxkNDzqRf+9m50QbKatXAbwX4EO5rprNaNfhx5sIyHkC0UQ2CLN9yHMdaymVMgWNSgyAViy9jBqINaiCWTmIVW8AxqMF4rqSInZxmINqoBi9W3+kmjmU74pRsgGNQY7zqfZf/+BykINqkRr3F4J8q0vZEcu4FLUBN9VvMv52SfqXojVRTX22U3E7JsCyoBWqMsknEMC77tP4HOKCmPgxD11qBUlTTHBTZUTQioloKapqmKx1R6i8qGtQ0jZ2IsCP7i3Gaq8Hzpi7t3rCkpnmW9BqhT+0/y9faJsxNMikAAAAASUVORK5CYII="
							}
							"#
							.to_owned(),
						},
					)
					.await?;
			}
			(State::Status, Ping::ID) => {
				let req = data.cast::<Ping>()?;
				stream
					.write_packet(
						None,
						&Pong {
							payload: req.payload,
						},
					)
					.await?;
			}
			(State::Login, LoginStart::ID) => {
				let req = data.cast::<LoginStart>()?;
				// TODO: Encryption
				break Ok((
					stream,
					LoggedInInfo {
						username: req.name, //req.name
						uuid: "530fa97a-357f-3c19-94d3-0c5c65c18fe8".into(),
					},
				));
			}
			(state, id) => break Err(SocketLoginError::IncorrectStateIdCombo(state, id)),
		}
	}
}
struct ConnectedServerInfo;
quick_error! {
	#[derive(Debug)]
	pub enum ServerConnectionError {
		Io(err: io::Error) {
			from()
			display("io error: {}", err)
			cause(err)
		}
		IncorrectStateIdCombo(state: State, id: i32) {}
		Disconnect(err: String) {}
		BadLoginSuccess(res: LoginSuccess) {}
		BadCompressionThreshold(threshold: Option<i32>) {}
	}
}

/// Открывает соединение с сервером для заданного юзера, проверяет корректность возвращённых данных
async fn open_server_connection(
	info: &LoggedInInfo,
) -> Result<(TcpStream, ConnectedServerInfo), ServerConnectionError> {
	let mut stream = TcpStream::connect("89.163.251.26:25738").await?;
	let mut buf = Vec::new();
	println!("Opening");

	let mut compression = None;
	stream
		.write_packet(
			compression,
			&Handshake {
				address: "mc.bullcraft.ru".to_owned(),
				protocol: 340.into(),
				port: 25565,
				next_state: State::Login,
			},
		)
		.await?;
	stream
		.write_packet(
			compression,
			&LoginStart {
				name: info.username.clone(),
			},
		)
		.await?;
	// Packet handling loop
	let state = State::Login;
	loop {
		let data = stream.read_packet(compression, &mut buf).await?;
		match (state, data.id()) {
			(State::Login, SetCompression::ID) => {
				let set_compression = data.cast::<SetCompression>()?;
				compression = Some(set_compression.threshold.0);
			}
			(State::Login, Disconnect::ID) => {
				let disconnect = data.cast::<Disconnect>()?;
				break Err(ServerConnectionError::Disconnect(disconnect.reason));
			}
			(State::Login, LoginSuccess::ID) => {
				let success = data.cast::<LoginSuccess>()?;
				println!("{:?}", success);
				if success.username != info.username || success.uuid != info.uuid {
					break Err(ServerConnectionError::BadLoginSuccess(success));
				}
				if let Some(THRESHOLD) = compression {
					break Ok((stream, ConnectedServerInfo));
				} else {
					break Err(ServerConnectionError::BadCompressionThreshold(compression));
				}
			}
			(state, id) => break Err(ServerConnectionError::IncorrectStateIdCombo(state, id)),
		};
	}
}
struct StreamPair {
	user: TcpStream,
	server: TcpStream,
}

/// Проводит общение юзера с сервером, успешно выходит после завершения соединения с сервером, падает при падении клиента
async fn communicate_user_server(streams: StreamPair) -> io::Result<TcpStream> {
	let compression = Some(THRESHOLD);
	let (mut server_read, mut server_write) = streams.server.into_split();
	let (mut user_read, mut user_write) = streams.user.into_split();
	let server = tokio::spawn(async move {
		let mut buf = Vec::new();
		loop {
			let packet = server_read
				.read_packet(compression, &mut buf)
				.await
				.unwrap();
			println!("S => C {:?}", packet);
			packet.write(compression, &mut user_write).await.unwrap();
		}
		user_write
	});
	let user = tokio::spawn(async move {
		let mut buf = Vec::new();
		loop {
			let packet = user_read.read_packet(compression, &mut buf).await.unwrap();
			println!("C => S {:?}", packet);
			packet.write(compression, &mut server_write).await.unwrap();
		}
		user_read
	});
	let user_write = server.await?;
	let user_read = user.await?;
	Ok(user_read.reunite(user_write).unwrap())
}

quick_error! {
	#[derive(Debug)]
	pub enum SocketError {
		Io(err: io::Error) {
			from()
		}
		Login(err: SocketLoginError) {
			from()
		}
		Server(err: ServerConnectionError) {
			from()
		}
	}
}

async fn handle_stream(stream: TcpStream) -> Result<(), SocketError> {
	let (mut user, logged_in) = handle_socket_login(stream).await?;
	println!("User logged in");
	let mut first_connection = true;
	loop {
		let (mut server, server_info) = open_server_connection(&logged_in).await?;

		if first_connection {
			user.write_packet(
				None,
				&SetCompression {
					threshold: THRESHOLD.into(),
				},
			)
			.await?;
			user.write_packet(
				Some(THRESHOLD),
				&LoginSuccess {
					username: logged_in.username.clone(),
					uuid: logged_in.uuid.clone(),
				},
			)
			.await?;
			first_connection = false;
		}
		println!("Server connected");
		user = communicate_user_server(StreamPair { user, server })
			.await
			.unwrap();
	}
}

#[tokio::main(core_threads = 4, max_threads = 8)]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	let mut listener = TcpListener::bind("127.0.0.1:25566").await?;

	loop {
		let (stream, _) = listener.accept().await?;
		println!("Got connection: {:?}", stream);
		tokio::spawn(async move {
			if let Err(e) = handle_stream(stream).await {
				println!("User error: {:?}", e);
			};
		});
	}
}
