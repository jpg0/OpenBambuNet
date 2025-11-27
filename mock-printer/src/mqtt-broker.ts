import Aedes from 'aedes';
import { createServer, Server as NetServer } from 'net';
import { createServer as createTlsServer, Server as TlsServer } from 'tls';
import { readFileSync } from 'fs';
import { PrinterState } from './printer-state';

export interface MqttBrokerConfig {
  port: number;
  useTls: boolean;
  certPath?: string;
  keyPath?: string;
}

/**
 * MQTT broker that simulates a Bambu Lab printer's MQTT interface.
 * Handles authentication, topic subscriptions, and auto-responds to commands.
 */
export class MqttBroker {
  private aedes: Aedes;
  private server: NetServer | TlsServer | null = null;
  private state: PrinterState;
  private config: MqttBrokerConfig;

  constructor(state: PrinterState, config: MqttBrokerConfig) {
    this.state = state;
    this.config = config;
    
    this.aedes = new Aedes();

    // Authentication
    this.aedes.authenticate = (client, username, password, callback) => {
      const printerConfig = this.state.getConfig();
      const validUsername = username?.toString() === 'bblp';
      const validPassword = password?.toString() === printerConfig.password;
      
      if (validUsername && validPassword) {
        console.log(`[MQTT] Client authenticated: ${client.id}`);
        callback(null, true);
      } else {
        console.log(`[MQTT] Authentication failed for ${client.id}`);
        const err: any = new Error('Authentication failed');
        err.returnCode = 4; // Bad username or password
        callback(err, false);
      }
    };

    // Client connection
    this.aedes.on('client', (client) => {
      console.log(`[MQTT] Client connected: ${client.id}`);
      this.state.setConnected(true);
    });

    // Client disconnection
    this.aedes.on('clientDisconnect', (client) => {
      console.log(`[MQTT] Client disconnected: ${client.id}`);
      this.state.setConnected(false);
    });

    // Subscribe events
    this.aedes.on('subscribe', (subscriptions, client) => {
      console.log(`[MQTT] Client ${client.id} subscribed to:`, 
        subscriptions.map(s => s.topic).join(', '));
    });

    // Published messages
    this.aedes.on('publish', (packet, client) => {
      if (!client) return; // Skip internal messages
      
      const topic = packet.topic;
      const payload = packet.payload.toString();
      
      console.log(`[MQTT] Message received on ${topic}`);
      
      // Track messages sent to the request topic
      const requestTopic = `device/${this.state.getConfig().devId}/request`;
      if (topic === requestTopic) {
        this.state.addReceivedMessage(topic, payload);
        
        // Auto-respond with status if enabled
        if (this.state.shouldAutoRespond()) {
          this.publishStatus();
        }
      }
    });
  }

  /**
   * Start the MQTT broker
   */
  async start(): Promise<void> {
    return new Promise((resolve, reject) => {
      try {
        if (this.config.useTls && this.config.certPath && this.config.keyPath) {
          // TLS server
          const tlsOptions = {
            key: readFileSync(this.config.keyPath),
            cert: readFileSync(this.config.certPath),
            rejectUnauthorized: false,
          };
          this.server = createTlsServer(tlsOptions, this.aedes.handle);
        } else {
          // Plain TCP server
          this.server = createServer(this.aedes.handle);
        }

        this.server.listen(this.config.port, () => {
          const protocol = this.config.useTls ? 'mqtts' : 'mqtt';
          console.log(`[MQTT] Broker listening on ${protocol}://0.0.0.0:${this.config.port}`);
          resolve();
        });

        this.server.on('error', (err) => {
          console.error('[MQTT] Server error:', err);
          reject(err);
        });
      } catch (err) {
        reject(err);
      }
    });
  }

  /**
   * Stop the MQTT broker
   */
  async stop(): Promise<void> {
    return new Promise((resolve) => {
      if (this.server) {
        this.server.close(() => {
          console.log('[MQTT] Broker stopped');
          resolve();
        });
        this.aedes.close(() => {
          console.log('[MQTT] Aedes closed');
        });
      } else {
        resolve();
      }
    });
  }

  /**
   * Publish a status message to the report topic
   */
  publishStatus(): void {
    const reportTopic = `device/${this.state.getConfig().devId}/report`;
    const statusMessage = this.state.generateStatusMessage();
    
    this.aedes.publish({
      cmd: 'publish',
      topic: reportTopic,
      payload: Buffer.from(statusMessage),
      qos: 0,
      dup: false,
      retain: false,
    } as any, (err) => {
      if (err) {
        console.error('[MQTT] Error publishing status:', err);
      } else {
        console.log(`[MQTT] Published status to ${reportTopic}`);
      }
    });
  }

  /**
   * Publish a custom message to the report topic
   */
  publishMessage(message: string): void {
    const reportTopic = `device/${this.state.getConfig().devId}/report`;
    
    this.aedes.publish({
      cmd: 'publish',
      topic: reportTopic,
      payload: Buffer.from(message),
      qos: 0,
      dup: false,
      retain: false,
    } as any, (err) => {
      if (err) {
        console.error('[MQTT] Error publishing message:', err);
      } else {
        console.log(`[MQTT] Published custom message to ${reportTopic}`);
      }
    });
  }
}
