//! Small, isolated MQTT publisher for read-only station status.
//!
//! The publisher owns its network threads. A broker outage therefore cannot
//! block the AX.25/runtime thread. Commands are bounded and stale snapshots
//! are discarded when the queue is full.
use crate::web_config::WebConfig;
use rumqttc::{Client, LastWill, MqttOptions, QoS, Transport};
use std::{sync::mpsc, thread, time::Duration};

pub struct Message {
    pub topic: String,
    pub payload: String,
    pub retain: bool,
}

enum Command {
    Snapshot(Vec<Message>),
    Stop,
}

pub struct Publisher {
    sender: mpsc::SyncSender<Command>,
    thread: Option<thread::JoinHandle<()>>,
}

impl Publisher {
    pub fn start(
        config: &WebConfig,
        password: String,
        callsign: String,
    ) -> Result<Option<Self>, String> {
        if !config.mqtt_enabled {
            return Ok(None);
        }
        let (host, port, tls) = parse_broker(&config.mqtt_broker, config.mqtt_tls)?;
        if config.mqtt_client_id.is_empty() || config.mqtt_topic_prefix.is_empty() {
            return Err("MQTT wymaga identyfikatora klienta i prefiksu tematów".into());
        }
        let mut options = MqttOptions::new(&config.mqtt_client_id, host, port);
        options.set_keep_alive(Duration::from_secs(30));
        if !config.mqtt_username.is_empty() {
            options.set_credentials(&config.mqtt_username, password);
        }
        if tls {
            options.set_transport(Transport::tls_with_default_config());
        }
        let status_topic = format!("{}/{callsign}/status", config.mqtt_topic_prefix);
        options.set_last_will(LastWill::new(
            status_topic,
            "{\"state\":\"offline\"}",
            QoS::AtLeastOnce,
            true,
        ));
        let (client, mut connection) = Client::new(options, 16);
        let (sender, receiver) = mpsc::sync_channel(2);
        let network = thread::Builder::new()
            .name("ultimatepr-mqtt".into())
            .spawn(move || {
                for command in receiver {
                    match command {
                        Command::Snapshot(messages) => {
                            for message in messages {
                                let _ = client.publish(
                                    message.topic,
                                    QoS::AtLeastOnce,
                                    message.retain,
                                    message.payload,
                                );
                            }
                        }
                        Command::Stop => break,
                    }
                }
            })
            .map_err(|e| format!("Nie można uruchomić klienta MQTT: {e}"))?;
        thread::Builder::new()
            .name("ultimatepr-mqtt-eventloop".into())
            .spawn(move || for _ in connection.iter() {})
            .map_err(|e| format!("Nie można uruchomić pętli MQTT: {e}"))?;
        Ok(Some(Self {
            sender,
            thread: Some(network),
        }))
    }

    pub fn publish(&self, messages: Vec<Message>) {
        if self.sender.try_send(Command::Snapshot(messages)).is_err() {
            // A newer status snapshot supersedes an older one.
        }
    }
}

impl Drop for Publisher {
    fn drop(&mut self) {
        let _ = self.sender.try_send(Command::Stop);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

fn parse_broker(value: &str, tls: bool) -> Result<(String, u16, bool), String> {
    let value = value.trim();
    let (value, scheme_tls) = if let Some(value) = value.strip_prefix("mqtt://") {
        (value, false)
    } else if let Some(value) = value.strip_prefix("mqtts://") {
        (value, true)
    } else {
        (value, tls)
    };
    let (host, port) = value
        .rsplit_once(':')
        .ok_or("Broker MQTT musi mieć postać host:port")?;
    let port = port.parse::<u16>().map_err(|_| "Nieprawidłowy port MQTT")?;
    if host.is_empty() || port == 0 {
        return Err("Nieprawidłowy adres brokera MQTT".into());
    }
    Ok((host.to_owned(), port, scheme_tls || tls))
}
