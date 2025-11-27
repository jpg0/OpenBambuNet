use paho_mqtt::{AsyncClient, ConnectOptionsBuilder, CreateOptionsBuilder, Message, SslOptionsBuilder};
use std::time::Duration;
use crate::printer::Printer;

#[derive(Clone)]
pub struct MqttClient {
    client: AsyncClient,
    printer: Printer,
}

impl MqttClient {
    pub async fn connect(printer: Printer) -> Result<Self, paho_mqtt::Error> {
        let _protocol = if printer.ssl { "ssl" } else { "tcp" }; // paho-mqtt uses ssl:// or tcp://
        // Wait, main.rs used "mqtts" and "mqtt". Let's check main.rs again.
        // main.rs: let protocol = if printer.ssl { "mqtts" } else { "mqtt" };
        // paho-mqtt supports these.
        
        let protocol = if printer.ssl { "mqtts" } else { "mqtt" };
        let uri = format!("{}://{}:{}", protocol, printer.ip, printer.mqtt_port);
        
        let create_opts = CreateOptionsBuilder::new()
            .server_uri(&uri)
            .client_id(format!("open-bambu-{}", printer.id)) // Add a client ID
            .finalize();

        let client = AsyncClient::new(create_opts)?;

        let ssl_opts = SslOptionsBuilder::new()
            .verify(false)
            .enable_server_cert_auth(false)
            .finalize();

        let conn_opts = ConnectOptionsBuilder::new()
            .user_name("bblp")
            .password(&printer.password)
            .ssl_options(ssl_opts)
            .keep_alive_interval(Duration::from_secs(20))
            .clean_session(true)
            .finalize();

        client.connect(conn_opts).await?;

        Ok(Self { client, printer })
    }

    pub async fn subscribe(&self) -> Result<(), paho_mqtt::Error> {
        let topic = format!("device/{}/report", self.printer.id);
        self.client.subscribe(topic, 0).await?;
        Ok(())
    }

    pub async fn publish(&self, payload: String) -> Result<(), paho_mqtt::Error> {
        let topic = format!("device/{}/request", self.printer.id);
        let msg = Message::new(topic, payload, 0);
        self.client.publish(msg).await?;
        Ok(())
    }
    
    pub fn get_stream(&mut self) -> paho_mqtt::AsyncReceiver<Option<Message>> {
        self.client.get_stream(25)
    }
    
    pub async fn disconnect(&self) -> Result<(), paho_mqtt::Error> {
        self.client.disconnect(None).await?;
        Ok(())
    }
}
