use lettre::smtp::client::net::NetworkStream;
use lettre::smtp::client::InnerClient;
use lettre::smtp::commands::*;
use lettre::smtp::extension::{ClientId, ServerInfo};
use lettre::EmailAddress;
use std::time::Duration;
use trust_dns_resolver::Name;

// Try to send an smtp command, close if fails.
macro_rules! try_smtp (
    ($res: expr, $client: ident, $host: expr, $port: expr) => ({
        match $res {
            Ok(res) => res,
            Err(err) => {
				debug!("Closing {}:{}, because of error '{}'.", $host, $port, err);
				$client.close();
                return None;
            },
        }
    })
);

pub fn email_exists(from: &str, to: &str, host: &Name, port: u16) -> Option<bool> {
	debug!("Connecting to {}:{}...", host, port);
	let mut smtp_client: InnerClient<NetworkStream> = InnerClient::new();

	// Set timeout.
	try_smtp!(
		smtp_client.set_timeout(Some(Duration::new(3, 0))),
		smtp_client,
		host,
		port
	);

	// Connect to the host.
	try_smtp!(
		smtp_client.connect(&(host.to_utf8().as_str(), port), None),
		smtp_client,
		host,
		port
	);

	// Send ehlo and get server info.
	let ehlo_response = try_smtp!(
		smtp_client.command(EhloCommand::new(ClientId::new("localhost".to_string()))),
		smtp_client,
		host,
		port
	);
	let server_info = ServerInfo::from_response(&ehlo_response);
	debug!("Server info: {}", server_info.as_ref().unwrap());

	// Send from.
	try_smtp!(
		smtp_client.command(MailCommand::new(
			Some(EmailAddress::new(from.to_string()).unwrap()),
			vec![],
		)),
		smtp_client,
		host,
		port
	);

	// Send to.
	let result = match smtp_client.command(RcptCommand::new(
		EmailAddress::new(to.to_string()).unwrap(),
		vec![],
	)) {
		Ok(response) => match response.first_line() {
			Some(message) => {
				// 250 2.1.0 Sender e-mail address ok.
				if message.contains("2.1.5") {
					Some(true)
				} else {
					None
				}
			}
			_ => None,
		},
		Err(err) => {
			// 550 5.1.1 Mailbox does not exist.
			if err.to_string().contains("5.1.1") {
				Some(false)
			} else {
				None
			}
		}
	};

	// Quit.
	smtp_client.close();

	match result {
		Some(val) => debug!("Checked email on {}:{}, exists={}.", host, port, val),
		None => debug!("Cannot check email on {}:{}.", host, port),
	};
	result
}
